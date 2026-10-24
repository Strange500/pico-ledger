use buoyant::{match_view, view::prelude::*};
use embedded_graphics::{mono_font::ascii::FONT_6X10, pixelcolor::BinaryColor, prelude::*};
use embedded_graphics_simulator::{OutputSettings, SimulatorDisplay, Window};

fn main() {
    let mut window = Window::new("Hello World", &OutputSettings::default());
    let mut display: SimulatorDisplay<BinaryColor> = SimulatorDisplay::new(Size::new(128, 64));
    display.clear(BinaryColor::Off);

    seed_phrase_view(&["apple", "banana", "cherry"])
        .as_drawable(display.size(), BinaryColor::On, &mut ())
        .draw(&mut display)
        .unwrap();

    window.show_static(&display);
}

fn home_view() -> impl View<BinaryColor, ()> {
    VStack::new((
        Text::new("Home", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        Text::new("Welcome to Pico Ledger!", &FONT_6X10),
        Spacer::default(),
        Text::new("Press > to start", &FONT_6X10),
    ))
    .padding(Edges::Horizontal, 4)
    .padding(Edges::Vertical, 2)
}

fn seed_phrase_view(words: &[&str]) -> impl View<BinaryColor, ()> {
    VStack::new((
        Text::new("Seed Phrase", &FONT_6X10),
        Divider::new(1),
        Spacer::default(),
        ForEach::<3>::new_vertical(words, |word| Text::new(word, &FONT_6X10)),
        Spacer::default(),
        Text::new("Press > to continue", &FONT_6X10),
    ))
    .padding(Edges::Horizontal, 4)
    .padding(Edges::Vertical, 2)
}

pub enum Screens {
    Home,
    SeedPhrase {
        words: [&'static str; 3],
        words_per_page: usize,
        page: usize,
    },
}

impl Screens {
    pub fn view(&self) -> impl View<BinaryColor, ()> {
        match_view!(
            self,
        {
            Screens::Home => home_view(),
            Screens::SeedPhrase {
                words,
                words_per_page,
                page,
            } => {
                let start = page * words_per_page;
                let end = (start + words_per_page).min(words.len());
                seed_phrase_view(&words[start..end])
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screens_fit_128x64() {
        let mut display: SimulatorDisplay<BinaryColor> = SimulatorDisplay::new(Size::new(128, 64));
        display.clear(BinaryColor::Off);
        home_view()
            .as_drawable(display.size(), BinaryColor::On, &mut ())
            .draw(&mut display)
            .unwrap();

        display.clear(BinaryColor::Off);
        seed_phrase_view(&["apple", "banana", "cherry"])
            .as_drawable(display.size(), BinaryColor::On, &mut ())
            .draw(&mut display)
            .unwrap();
    }
}
