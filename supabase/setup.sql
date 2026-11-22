-- AimForge leaderboards. Paste all of this into Supabase -> SQL Editor -> Run.
-- Safe to run again.

create table if not exists public.scores (
  id            bigint generated always as identity primary key,
  created_at    timestamptz not null default now(),
  scenario      text not null check (char_length(scenario) between 1 and 120),
  scenario_name text not null check (char_length(scenario_name) between 1 and 120),
  player        text not null check (char_length(player) between 1 and 24),
  player_id     uuid not null,
  score         real not null check (score between -100000 and 1000000),
  accuracy      real not null check (accuracy between 0 and 1),
  kills         int  not null default 0 check (kills between 0 and 100000),
  hits          int  not null default 0 check (hits >= 0),
  shots         int  not null default 0 check (shots >= 0),
  cm360         real,
  hfov          real,
  duration      real,
  game_version  text check (game_version is null or char_length(game_version) < 20),
  replay_path   text check (replay_path is null or char_length(replay_path) < 300)
);
create index if not exists scores_scenario_score on public.scores (scenario, score desc);

-- Anyone may read and add scores; nobody can edit or delete through the API.
alter table public.scores enable row level security;
grant select, insert on public.scores to anon, authenticated;
drop policy if exists "read scores" on public.scores;
create policy "read scores" on public.scores for select to anon, authenticated using (true);
drop policy if exists "add scores" on public.scores;
create policy "add scores" on public.scores for insert to anon, authenticated with check (true);

-- Best run per player per scenario.
create or replace view public.leaderboard with (security_invoker = on) as
  select distinct on (scenario, player_id) *
  from public.scores
  order by scenario, player_id, score desc, created_at asc;
grant select on public.leaderboard to anon, authenticated;

-- Replay files (max 2 MB each), publicly readable.
insert into storage.buckets (id, name, public, file_size_limit)
values ('replays', 'replays', true, 2097152)
on conflict (id) do nothing;
drop policy if exists "read replays" on storage.objects;
create policy "read replays" on storage.objects for select to anon, authenticated using (bucket_id = 'replays');
drop policy if exists "upload replays" on storage.objects;
create policy "upload replays" on storage.objects for insert to anon, authenticated with check (bucket_id = 'replays');
