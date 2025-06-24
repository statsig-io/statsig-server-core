use std::collections::VecDeque;

pub struct WindowIter<'a> {
    iter: std::str::SplitWhitespace<'a>,
    window: VecDeque<&'a str>,
}

type Window<'a> = (
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
);

impl<'a> WindowIter<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut iter = input.split_whitespace();
        let mut window = VecDeque::new();

        for _ in 0..4 {
            if let Some(word) = iter.next() {
                window.push_back(word);
            }
        }

        Self { iter, window }
    }

    #[allow(clippy::get_first)]
    pub fn get_window(&self) -> Window<'a> {
        (
            self.window.get(0).copied(),
            self.window.get(1).copied(),
            self.window.get(2).copied(),
            self.window.get(3).copied(),
        )
    }

    pub fn slide_window_by(&mut self, n: usize) {
        for _ in 0..n {
            self.window.pop_front();
            if let Some(word) = self.iter.next() {
                self.window.push_back(word);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }
}
