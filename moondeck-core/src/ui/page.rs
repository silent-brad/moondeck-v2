use super::{Event, Gesture, WidgetInstance};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub widgets: Vec<WidgetInstance>,
    #[serde(default)]
    pub background_color: Option<String>,
}

impl Page {
    pub fn new(id: &str, title: &str) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            widgets: Vec::new(),
            background_color: None,
        }
    }

    pub fn with_widget(mut self, widget: WidgetInstance) -> Self {
        self.widgets.push(widget);
        self
    }

    pub fn with_background(mut self, color: &str) -> Self {
        self.background_color = Some(color.to_string());
        self
    }
}

pub struct PageManager {
    pages: Vec<Page>,
    current_index: usize,
    #[allow(dead_code)]
    transition_progress: f32,
    #[allow(dead_code)]
    transitioning_to: Option<usize>,
}

impl PageManager {
    pub fn new() -> Self {
        Self {
            pages: Vec::new(),
            current_index: 0,
            transition_progress: 0.0,
            transitioning_to: None,
        }
    }

    pub fn with_pages(mut self, pages: Vec<Page>) -> Self {
        self.pages = pages;
        self
    }

    pub fn add_page(&mut self, page: Page) {
        self.pages.push(page);
    }

    pub fn current_page(&self) -> Option<&Page> {
        self.pages.get(self.current_index)
    }

    pub fn current_page_mut(&mut self) -> Option<&mut Page> {
        self.pages.get_mut(self.current_index)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn go_to(&mut self, index: usize) {
        if index < self.pages.len() {
            self.current_index = index;
        }
    }

    pub fn next_page(&mut self) {
        if !self.pages.is_empty() {
            self.current_index = (self.current_index + 1) % self.pages.len();
        }
    }

    pub fn prev_page(&mut self) {
        if !self.pages.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.pages.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Gesture(Gesture::SwipeLeft) => {
                self.next_page();
                true
            }
            Event::Gesture(Gesture::SwipeRight) => {
                self.prev_page();
                true
            }
            _ => false,
        }
    }

    pub fn pages(&self) -> &[Page] {
        &self.pages
    }

    pub fn pages_mut(&mut self) -> &mut [Page] {
        &mut self.pages
    }
}

impl Default for PageManager {
    fn default() -> Self {
        Self::new()
    }
}
