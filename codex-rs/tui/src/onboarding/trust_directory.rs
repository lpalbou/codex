use std::path::PathBuf;

use codex_config::CONFIG_TOML_FILE;
use codex_config::default_project_root_markers;
use codex_config::find_project_root;
use codex_config::project_root_markers_from_config;
use codex_core::config::set_project_trust_level;
use codex_git_utils::resolve_root_git_project_for_trust;
use codex_protocol::config_types::TrustLevel;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::widgets::WidgetRef;
use ratatui::widgets::Wrap;

use crate::key_hint;
use crate::onboarding::onboarding_screen::KeyboardHandler;
use crate::onboarding::onboarding_screen::StepStateProvider;
use crate::render::Insets;
use crate::render::renderable::ColumnRenderable;
use crate::render::renderable::Renderable;
use crate::render::renderable::RenderableExt as _;
use crate::selection_list::selection_option_row;

use super::onboarding_screen::StepState;
pub(crate) struct TrustDirectoryWidget {
    pub codex_home: PathBuf,
    pub cwd: PathBuf,
    pub show_windows_create_sandbox_hint: bool,
    pub should_quit: bool,
    pub selection: Option<TrustDirectorySelection>,
    pub highlighted: TrustDirectorySelection,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustDirectorySelection {
    Trust,
    Quit,
}

impl WidgetRef for &TrustDirectoryWidget {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let mut column = ColumnRenderable::new();

        column.push(Line::from(vec![
            "> ".into(),
            "You are in ".bold(),
            self.cwd.to_string_lossy().to_string().into(),
        ]));
        column.push("");

        column.push(
            Paragraph::new(
                "Do you trust the contents of this directory? Working with untrusted contents comes with higher risk of prompt injection.".to_string(),
            )
                .wrap(Wrap { trim: true })
                .inset(Insets::tlbr(/*top*/ 0, /*left*/ 2, /*bottom*/ 0, /*right*/ 0)),
        );
        column.push("");

        let options: Vec<(&str, TrustDirectorySelection)> = vec![
            ("Yes, continue", TrustDirectorySelection::Trust),
            ("No, quit", TrustDirectorySelection::Quit),
        ];

        for (idx, (text, selection)) in options.iter().enumerate() {
            column.push(selection_option_row(
                idx,
                text.to_string(),
                self.highlighted == *selection,
            ));
        }

        column.push("");

        if let Some(error) = &self.error {
            column.push(
                Paragraph::new(error.to_string())
                    .red()
                    .wrap(Wrap { trim: true })
                    .inset(Insets::tlbr(
                        /*top*/ 0, /*left*/ 2, /*bottom*/ 0, /*right*/ 0,
                    )),
            );
            column.push("");
        }

        column.push(
            Line::from(vec![
                "Press ".dim(),
                key_hint::plain(KeyCode::Enter).into(),
                if self.show_windows_create_sandbox_hint {
                    " to continue and create a sandbox...".dim()
                } else {
                    " to continue".dim()
                },
            ])
            .inset(Insets::tlbr(
                /*top*/ 0, /*left*/ 2, /*bottom*/ 0, /*right*/ 0,
            )),
        );

        column.render(area, buf);
    }
}

impl KeyboardHandler for TrustDirectoryWidget {
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        if key_event.kind == KeyEventKind::Release {
            return;
        }

        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.highlighted = TrustDirectorySelection::Trust;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.highlighted = TrustDirectorySelection::Quit;
            }
            KeyCode::Char('1') | KeyCode::Char('y') => self.handle_trust(),
            KeyCode::Char('2') | KeyCode::Char('n') => self.handle_quit(),
            KeyCode::Enter => match self.highlighted {
                TrustDirectorySelection::Trust => self.handle_trust(),
                TrustDirectorySelection::Quit => self.handle_quit(),
            },
            _ => {}
        }
    }
}

impl StepStateProvider for TrustDirectoryWidget {
    fn get_step_state(&self) -> StepState {
        if self.selection.is_some() || self.should_quit {
            StepState::Complete
        } else {
            StepState::InProgress
        }
    }
}

impl TrustDirectoryWidget {
    fn handle_trust(&mut self) {
        let target = trust_target_for_cwd(&self.codex_home, &self.cwd);
        if let Err(e) = set_project_trust_level(&self.codex_home, &target, TrustLevel::Trusted) {
            tracing::error!("Failed to set project trusted: {e:?}");
            self.error = Some(format!("Failed to set trust for {}: {e}", target.display()));
        }

        self.selection = Some(TrustDirectorySelection::Trust);
    }

    fn handle_quit(&mut self) {
        self.highlighted = TrustDirectorySelection::Quit;
        self.should_quit = true;
    }

    pub fn should_quit(&self) -> bool {
        self.should_quit
    }
}

fn trust_target_for_cwd(codex_home: &std::path::Path, cwd: &std::path::Path) -> PathBuf {
    if let Some(repo_root) = resolve_root_git_project_for_trust(cwd) {
        return repo_root;
    }

    let config_file = codex_home.join(CONFIG_TOML_FILE);
    let project_root_markers = std::fs::read_to_string(&config_file)
        .ok()
        .and_then(|contents| toml::from_str(&contents).ok())
        .and_then(|config| match project_root_markers_from_config(&config) {
            Ok(markers) => markers,
            Err(err) => {
                tracing::warn!(
                    "Failed to parse project_root_markers from {}: {err}",
                    config_file.display()
                );
                None
            }
        })
        .unwrap_or_else(default_project_root_markers);

    find_project_root(cwd, &project_root_markers)
}

#[cfg(test)]
mod tests {
    use crate::test_backend::VT100Backend;

    use super::*;
    use crossterm::event::KeyCode;
    use crossterm::event::KeyEvent;
    use crossterm::event::KeyEventKind;
    use crossterm::event::KeyModifiers;
    use pretty_assertions::assert_eq;
    use ratatui::Terminal;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn release_event_does_not_change_selection() {
        let codex_home = TempDir::new().expect("temp home");
        let mut widget = TrustDirectoryWidget {
            codex_home: codex_home.path().to_path_buf(),
            cwd: PathBuf::from("."),
            show_windows_create_sandbox_hint: false,
            should_quit: false,
            selection: None,
            highlighted: TrustDirectorySelection::Quit,
            error: None,
        };

        let release = KeyEvent {
            kind: KeyEventKind::Release,
            ..KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        };
        widget.handle_key_event(release);
        assert_eq!(widget.selection, None);

        let press = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        widget.handle_key_event(press);
        assert!(widget.should_quit);
    }

    #[test]
    fn renders_snapshot_for_git_repo() {
        let codex_home = TempDir::new().expect("temp home");
        let widget = TrustDirectoryWidget {
            codex_home: codex_home.path().to_path_buf(),
            cwd: PathBuf::from("/workspace/project"),
            show_windows_create_sandbox_hint: false,
            should_quit: false,
            selection: None,
            highlighted: TrustDirectorySelection::Trust,
            error: None,
        };

        let mut terminal =
            Terminal::new(VT100Backend::new(/*width*/ 70, /*height*/ 14)).expect("terminal");
        terminal
            .draw(|f| (&widget).render_ref(f.area(), f.buffer_mut()))
            .expect("draw");

        insta::assert_snapshot!(terminal.backend());
    }

    #[test]
    fn handle_trust_persists_hg_project_root_from_configured_markers() {
        let codex_home = TempDir::new().expect("temp home");
        let project_root = codex_home.path().join("project");
        let nested = project_root.join("child");
        std::fs::create_dir_all(&nested).expect("create nested dir");
        std::fs::write(project_root.join(".hg"), "").expect("write hg marker");
        std::fs::write(
            codex_home.path().join(CONFIG_TOML_FILE),
            "project_root_markers = [\".hg\"]\n",
        )
        .expect("write config");

        let mut widget = TrustDirectoryWidget {
            codex_home: codex_home.path().to_path_buf(),
            cwd: nested,
            show_windows_create_sandbox_hint: false,
            should_quit: false,
            selection: None,
            highlighted: TrustDirectorySelection::Trust,
            error: None,
        };

        widget.handle_trust();

        let config_contents =
            std::fs::read_to_string(codex_home.path().join(CONFIG_TOML_FILE)).expect("read config");
        let trusted_project_header = format!("[projects.\"{}\"]", project_root.display());
        assert!(
            config_contents.contains(&trusted_project_header),
            "expected config to contain {trusted_project_header}, got:\n{config_contents}"
        );
        assert!(
            config_contents.contains("trust_level = \"trusted\""),
            "expected trusted config entry, got:\n{config_contents}"
        );
        assert_eq!(widget.selection, Some(TrustDirectorySelection::Trust));
        assert_eq!(widget.error, None);
    }
}
