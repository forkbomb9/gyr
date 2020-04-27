use fuzzy_filter::matches;

use tui::style::{Color, Style};
use tui::widgets::Text;

use super::apps;

pub struct UI<'a> {
    pub hidden: Vec<apps::Application>,
    pub shown: Vec<apps::Application>,
    pub selected: Option<usize>,
    pub text: Vec<Text<'a>>,
    pub query: String,
    pub log: Vec<Text<'a>>,
}

impl<'a> UI<'a> {
    pub fn new(items: Vec<apps::Application>) -> UI<'a> {
        UI {
            shown: items,
            hidden: vec![],
            selected: Some(0),
            text: vec![],
            query: String::new(),
            log: vec![],
        }
    }

    pub fn update_info(&mut self, color: Color) {
        if let Some(selected) = self.selected {
            self.text = vec![
                Text::styled(
                    format!("{}\n\n", &self.shown[selected].name),
                    Style::default().fg(color),
                ),
                Text::raw(format!("{}\n", &self.shown[selected].description)),
                if self.shown[selected].terminal_exec {
                    Text::raw("\nExec (terminal): ")
                } else {
                    Text::raw("\nExec: ")
                },
                Text::styled(
                    self.shown[selected].exec.to_string(),
                    Style::default().fg(Color::DarkGray),
                ),
            ];
        } else {
            self.text.clear();
        }
    }

    pub fn update_filter(&mut self) {
        let mut i = 0;
        while i != self.shown.len() {
            if !matches(&self.query, &self.shown[i].name.to_lowercase()) {
                self.hidden.push(self.shown.remove(i));
            } else {
                i += 1;
            }
        }

        for item in &self.hidden {
            if matches(&self.query, &item.name.to_lowercase()) && !self.shown.contains(item) {
                self.shown.push(item.clone());
            }
        }

        self.shown.sort();

        if self.shown.is_empty() {
            self.selected = None;
            self.log.push(Text::raw("NO ITEMS!"));
        } else {
            self.selected = Some(0);
        }

        self.log.push(Text::raw("update_filter\n"));
    }
}
