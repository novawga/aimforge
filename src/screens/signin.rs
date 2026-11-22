//! Sign in / create account, shown at launch until the player signs in or picks guest.

use super::intro::{draw_flicks, draw_glow, draw_range, Scene};
use crate::app::{App, Screen, SignInMode, SubmitState};
use crate::theme::{self, mix, with_alpha};
use crate::ui::{hash_id, rrect, size, Kind};
use macroquad::prelude::*;

const F_NAME: &str = "si_name";
const F_PASS: &str = "si_pass";
const F_PASS2: &str = "si_pass2";

impl App {
    /// Open the sign-in screen, coming back to `back_to` afterwards (None = menu).
    pub fn open_sign_in(&mut self, back_to: Option<Screen>) {
        self.signin.back_to = back_to;
        self.signin.error = None;
        self.signin.pass.clear();
        self.signin.pass2.clear();
        self.signin.autofocus = true;
        self.screen = Screen::SignIn;
        self.set_grab(false);
    }

    pub fn sign_out(&mut self) {
        self.settings.account = false;
        self.settings.guest = false;
        self.settings.player_name.clear();
        self.settings.player_id = crate::leaderboard::new_player_id();
        self.settings_dirty = true;
        self.signin = Default::default();
        self.open_sign_in(None);
    }

    /// Called with the server's answer.
    pub fn signed_in(&mut self, result: Result<(String, String), String>) {
        self.signin.busy = false;
        match result {
            Ok((id, name)) => {
                self.settings.player_id = id;
                self.settings.player_name = name.clone();
                self.settings.account = true;
                self.settings.guest = false;
                self.settings.submit_scores = true;
                self.settings_dirty = true;
                self.signin.pass.clear();
                self.signin.pass2.clear();
                self.signin.error = None;
                self.ui.focus = None;
                self.screen = self.signin.back_to.take().unwrap_or(Screen::Menu);
                self.toast(format!("Signed in as {name}"));
                // A run that finished while signed out goes up now.
                let pending = self.result.as_ref().filter(|r| r.submit == SubmitState::NeedName && r.rec.shots > 0);
                if let Some(r) = pending {
                    let (n, rec) = (r.name.clone(), r.rec.clone());
                    let bytes = r.replay.as_ref().map(|rp| rp.encode());
                    self.submit_run(&n, &rec, bytes);
                    if let Some(r) = &mut self.result {
                        r.submit = SubmitState::Sending;
                    }
                }
            }
            Err(e) => {
                let name_problem = e.to_lowercase().contains("name") && !e.contains("password");
                if name_problem {
                    self.ui.focus = Some(hash_id(F_NAME));
                } else {
                    if self.signin.mode == SignInMode::SignIn {
                        self.signin.pass.clear();
                    }
                    self.ui.focus = Some(hash_id(F_PASS));
                }
                self.signin.error = Some(e);
            }
        }
    }

    fn continue_as_guest(&mut self) {
        self.settings.guest = true;
        self.settings_dirty = true;
        self.ui.focus = None;
        self.screen = self.signin.back_to.take().unwrap_or(Screen::Menu);
    }

    fn submit_sign_in(&mut self) {
        let st = &mut self.signin;
        if st.busy {
            return;
        }
        let name = crate::leaderboard::clean_name(&st.name);
        st.error = if name.chars().count() < 2 {
            Some("Enter your name".into())
        } else if st.pass.is_empty() {
            Some("Enter your password".into())
        } else if st.mode == SignInMode::Create && st.pass.chars().count() < 6 {
            Some("Password must be at least 6 characters".into())
        } else if st.mode == SignInMode::Create && st.pass != st.pass2 {
            Some("Passwords don't match".into())
        } else {
            None
        };
        if let Some(e) = &st.error {
            let field = if e.contains("name") {
                F_NAME
            } else if e.contains("match") {
                F_PASS2
            } else {
                F_PASS
            };
            self.ui.focus = Some(hash_id(field));
            return;
        }
        st.busy = true;
        let create = st.mode == SignInMode::Create;
        let pass = st.pass.clone();
        self.board.account(create, &self.settings.player_id, &name, &pass);
    }

    pub fn frame_signin(&mut self) {
        let th = theme::t();
        let (sw, sh) = (screen_width(), screen_height());
        let s = self.ui.scale;
        let sc = |v: f32| (v * s).round();
        let t = get_time();
        clear_background(th.bg);

        // Backdrop: the loading screen's range, calmer and dimmed.
        draw_range(t * 0.6, 0.55, sh * 0.62);
        let ball = self.settings.map.ball;
        let ball = Color::from_rgba(ball[0], ball[1], ball[2], 255);
        draw_glow(vec2(sw * 0.5, sh * 0.42), sh * 0.5, th.accent, 0.8);
        draw_flicks(t, &Scene { center: vec2(sw * 0.5, sh * 0.42), radius: sh * 0.3, alpha: 0.28, ball, speed: 0.55 });

        // Card.
        let create = self.signin.mode == SignInMode::Create;
        let cw = sc(440.0);
        let err_lines = self.signin.error.as_ref().map(|e| self.ui.wrap(e, size::SMALL, cw - sc(72.0), false).len().min(2)).unwrap_or(0);
        let sub = if create { "Create an account to put your runs on the global leaderboards." } else { "Sign in to put your runs on the global leaderboards." };
        let sub_lines = self.ui.wrap(sub, size::SMALL, cw - sc(72.0), false);
        let ch = sc(470.0 + if create { 76.0 } else { 0.0 } + err_lines as f32 * 18.0 + (sub_lines.len().max(1) - 1) as f32 * 20.0);
        let card = Rect::new(sw * 0.5 - cw * 0.5, sh * 0.5 - ch * 0.5, cw, ch);
        rrect(Rect::new(card.x - sc(1.0), card.y - sc(1.0), card.w + sc(2.0), card.h + sc(2.0)), sc(13.0), with_alpha(th.text, 0.07));
        rrect(card, sc(12.0), with_alpha(mix(th.surface, th.bg, 0.25), 0.94));
        let pad = sc(36.0);
        let x = card.x + pad;
        let w = card.w - pad * 2.0;
        let mut y = card.y + pad;

        // Wordmark.
        let word = "AIMFORGE";
        let spacing = sc(6.0);
        let mut wx = x;
        for (i, c) in word.chars().enumerate() {
            let col = if i < 3 { th.text } else { mix(th.text, th.accent, 0.55) };
            wx += self.ui.strong(&c.to_string(), wx, y, size::TITLE, col) + spacing;
        }
        y += sc(46.0);
        for line in &sub_lines {
            self.ui.text(line, x, y, size::SMALL, th.text_dim);
            y += sc(20.0);
        }
        y += sc(14.0);

        let idx = if create { 1 } else { 0 };
        if let Some(i) = self.ui.segmented(Rect::new(x, y, w, sc(36.0)), &["Sign in", "Create account"], idx) {
            self.signin.mode = if i == 0 { SignInMode::SignIn } else { SignInMode::Create };
            self.signin.error = None;
            self.signin.pass2.clear();
            self.signin.autofocus = true;
        }
        y += sc(56.0);

        // Fields.
        let fh = sc(42.0);
        let mut submit = false;
        self.ui.text("Name", x, y, size::CAPTION, th.text_faint);
        y += sc(20.0);
        submit |= self.ui.text_field(hash_id(F_NAME), Rect::new(x, y, w, fh), &mut self.signin.name, "Your player name") && is_enter();
        y += fh + sc(14.0);
        self.ui.text("Password", x, y, size::CAPTION, th.text_faint);
        y += sc(20.0);
        let ph = if create { "At least 6 characters" } else { "Password" };
        submit |= self.ui.password_field(hash_id(F_PASS), Rect::new(x, y, w, fh), &mut self.signin.pass, ph) && is_enter();
        y += fh + sc(14.0);
        if create {
            self.ui.text("Confirm password", x, y, size::CAPTION, th.text_faint);
            y += sc(20.0);
            submit |= self.ui.password_field(hash_id(F_PASS2), Rect::new(x, y, w, fh), &mut self.signin.pass2, "Type it again") && is_enter();
            y += fh + sc(14.0);
        }
        // Tab moves between fields; focus the name field when nothing is focused.
        let order: Vec<u64> = if create { vec![F_NAME, F_PASS, F_PASS2] } else { vec![F_NAME, F_PASS] }.into_iter().map(hash_id).collect();
        if is_key_pressed(KeyCode::Tab) {
            let cur = self.ui.focus.and_then(|f| order.iter().position(|o| *o == f));
            let back = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            let n = order.len();
            let next = match cur {
                Some(i) if back => (i + n - 1) % n,
                Some(i) => (i + 1) % n,
                None => 0,
            };
            self.ui.focus = Some(order[next]);
        }
        if self.signin.autofocus {
            self.signin.autofocus = false;
            let first_empty = if self.signin.name.is_empty() { 0 } else { 1 };
            self.ui.focus = Some(order[first_empty]);
        }

        // Error line.
        if let Some(e) = &self.signin.error {
            let lines = self.ui.wrap(e, size::SMALL, w, false);
            for l in lines.iter().take(2) {
                self.ui.text(l, x, y - sc(4.0), size::SMALL, th.bad);
                y += sc(18.0);
            }
        }
        y += sc(6.0);

        let label = match (self.signin.busy, create) {
            (true, false) => "Signing in...",
            (true, true) => "Creating account...",
            (false, false) => "Sign in",
            (false, true) => "Create account",
        };
        let btn = Rect::new(x, y, w, sc(46.0));
        if self.ui.button(btn, label, Kind::Primary) {
            submit = true;
        }
        if self.signin.busy {
            // little spinner on the button
            let a = (t * 6.0) as f32;
            let c = vec2(btn.x + btn.w - sc(26.0), btn.y + btn.h * 0.5);
            for i in 0..3 {
                let ang = a + i as f32 * 2.1;
                draw_circle(c.x + ang.cos() * sc(7.0), c.y + ang.sin() * sc(7.0), sc(2.2), with_alpha(th.bg, 0.8));
            }
        }
        y += sc(58.0);
        if submit {
            self.submit_sign_in();
        }

        // Guest.
        let g = "Play as guest";
        let gw = self.ui.measure(g, size::SMALL, true);
        let gr = Rect::new(card.x + card.w * 0.5 - gw * 0.5 - sc(8.0), y, gw + sc(16.0), sc(28.0));
        let hov = self.ui.hover(gr);
        if hov {
            self.ui.hand = true;
        }
        self.ui.text_mid(g, gr.x + sc(8.0), gr, size::SMALL, if hov { th.text } else { th.text_dim }, true);
        if self.ui.click_in(gr) && !self.signin.busy {
            self.continue_as_guest();
            return;
        }
        self.ui.text_center("Guest runs stay on this PC", card.x + card.w * 0.5, y + sc(30.0), size::CAPTION, th.text_faint, false);

        if !self.board.online() {
            self.ui.text_center("Online features are off in this build", sw * 0.5, card.y + card.h + sc(18.0), size::CAPTION, th.text_faint, false);
        }
        self.ui.text_right(
            &format!("v{}", crate::update::VERSION.trim_start_matches('v')),
            sw - sc(24.0),
            sh - sc(36.0),
            size::CAPTION,
            th.text_faint,
            false,
        );
        if is_key_pressed(KeyCode::Escape) && self.ui.focus.is_none() && !self.ui.focus_at_begin && self.signin.back_to.is_some() {
            self.screen = self.signin.back_to.take().unwrap_or(Screen::Menu);
        }
    }
}

fn is_enter() -> bool {
    is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter)
}
