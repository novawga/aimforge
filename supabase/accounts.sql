-- AimForge accounts (v0.5+). Paste into Supabase -> SQL Editor -> Run, after setup.sql.
-- Safe to run again.
--
-- Players pick a name and password in the game. Passwords are stored as bcrypt
-- hashes, and the table itself is closed to the public API: the game can only
-- reach it through the two functions below.

create extension if not exists pgcrypto with schema extensions;

create table if not exists public.players (
  id         uuid primary key,
  name       text not null check (char_length(name) between 2 and 24),
  name_key   text not null unique,
  pass_hash  text not null,
  created_at timestamptz not null default now(),
  last_login timestamptz
);
alter table public.players enable row level security;
revoke all on public.players from anon, authenticated;

-- Create an account. Returns {id, name}. Errors are shown to the player as-is.
create or replace function public.register_player(p_id uuid, p_name text, p_pass text)
returns json
language plpgsql
security definer
set search_path = public, extensions
as $$
declare
  n   text := regexp_replace(trim(coalesce(p_name, '')), '\s+', ' ', 'g');
  key text := lower(n);
  pid uuid := coalesce(p_id, gen_random_uuid());
begin
  if char_length(n) < 2 or char_length(n) > 24 then
    raise exception 'Name must be 2 to 24 characters';
  end if;
  if n !~ '^[A-Za-z0-9 _.\-]+$' then
    raise exception 'Use letters, numbers, spaces, _ . or - in your name';
  end if;
  if char_length(coalesce(p_pass, '')) < 6 then
    raise exception 'Password must be at least 6 characters';
  end if;
  if exists (select 1 from players where name_key = key) then
    raise exception 'That name is taken';
  end if;
  if exists (select 1 from players where id = pid) then
    pid := gen_random_uuid();
  end if;
  -- Names already on the board before accounts existed belong to whoever posted them.
  if exists (select 1 from scores where lower(player) = key and player_id <> pid) then
    raise exception 'That name is taken';
  end if;
  insert into players (id, name, name_key, pass_hash, last_login)
  values (pid, n, key, crypt(p_pass, gen_salt('bf', 10)), now());
  return json_build_object('id', pid, 'name', n);
end;
$$;

-- Sign in. Returns {id, name}.
create or replace function public.login_player(p_name text, p_pass text)
returns json
language plpgsql
security definer
set search_path = public, extensions
as $$
declare
  r players%rowtype;
begin
  select * into r from players
  where name_key = lower(regexp_replace(trim(coalesce(p_name, '')), '\s+', ' ', 'g'));
  if not found or r.pass_hash <> crypt(coalesce(p_pass, ''), r.pass_hash) then
    perform pg_sleep(0.4);
    raise exception 'Wrong name or password';
  end if;
  update players set last_login = now() where id = r.id;
  return json_build_object('id', r.id, 'name', r.name);
end;
$$;

revoke all on function public.register_player(uuid, text, text) from public;
revoke all on function public.login_player(text, text) from public;
grant execute on function public.register_player(uuid, text, text) to anon, authenticated;
grant execute on function public.login_player(text, text) to anon, authenticated;

-- Make PostgREST see the new functions right away.
notify pgrst, 'reload schema';
