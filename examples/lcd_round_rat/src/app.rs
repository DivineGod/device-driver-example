use cst816s_device_driver::{device, CST816S};
use embedded_hal::{
    digital::{InputPin, OutputPin},
    i2c::I2c,
};

use core::fmt::Write;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    // style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    Frame,
    // Terminal,
};

use crate::EmbeddedTerminal;

pub struct App<A, B, C> {
    counter: u8,
    exit: bool,
    touchpad: CST816S<A, B, C>,
}

impl<A: I2c, B: InputPin, C: OutputPin> App<A, B, C> {
    pub fn new(touchpad: CST816S<A, B, C>) -> Self {
        Self {
            counter: 0,
            exit: false,
            touchpad,
        }
    }
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut EmbeddedTerminal) -> Result<(), mousefood::error::Error> {
        terminal.draw(|frame| self.draw(frame))?;
        self.handle_events();
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> Result<(), ()>
    where
        A: I2c,
        B: InputPin,
        C: OutputPin,
    {
        if let Some(touch_event) = self.touchpad.event() {
            match touch_event.gesture {
                device::Gesture::SlideUp => self.counter += 1,
                device::Gesture::SlideDown => self.counter -= 1,
                device::Gesture::SlideLeft => self.counter -= 1,
                device::Gesture::SlideRight => self.counter += 1,
                device::Gesture::SingleClick => {
                    if touch_event.point.0 <= 120 {
                        self.counter -= 1;
                    } else {
                        self.counter += 1;
                    }
                }
                device::Gesture::DoubleClick => {
                    if touch_event.point.0 <= 120 {
                        self.counter -= 10;
                    } else {
                        self.counter += 10;
                    }
                }
                device::Gesture::LongPress => self.counter = 0,
                _ => {}
            };
        }
        Ok(())
    }
}

impl<A, B, C> Widget for &App<A, B, C> {
    fn render(self, area: Rect, buf: &mut Buffer)
    where
        Self: Sized,
    {
        // `write` for `heapless::String` returns an error if the buffer is full,
        // but because the buffer here is 9 bytes large, the `(xxx:yyy)` will fit.
        let title = Line::from("Touch Counter");
        let mut data = heapless::String::<3>::new(); // 9 byte string buffer
        let counter = self.counter;
        let _ = write!(data, "{counter:03}").unwrap();

        let block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);

        Paragraph::new(Text::from(data.as_str()))
            .centered()
            .block(block)
            .render(area, buf);
    }
}
