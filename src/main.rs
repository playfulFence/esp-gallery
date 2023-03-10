#![no_std]
#![no_main]

#[cfg(feature="esp32")]
use esp32_hal as hal;
#[cfg(feature="esp32s2")]
use esp32s2_hal as hal;
#[cfg(feature="esp32s3")]
use esp32s3_hal as hal;
#[cfg(feature="esp32c3")]
use esp32c3_hal as hal;

use hal::{
    adc::{AdcConfig, Attenuation, ADC, ADC2},
    clock::ClockControl,
    peripherals::Peripherals,
    gpio::*,
    prelude::*,
    spi,
    timer::TimerGroup,
    Rtc,
    IO,
    Delay,
};

use mipidsi::Orientation;

use display_interface_spi::SPIInterfaceNoCS;

use core::f32::consts::PI;
use libm::{sin, cos};

use embedded_graphics::{
    prelude::RgbColor,
    mono_font::{
        ascii::FONT_10X20,
        MonoTextStyleBuilder,
        MonoTextStyle,
    },
    prelude::*,
    text::{Alignment, Text},
    Drawable,
    pixelcolor::*,
    primitives::{Circle, PrimitiveStyleBuilder, PrimitiveStyle, Rectangle},
    text::*,
    image::Image,
    geometry::*,
    draw_target::DrawTarget,
};

use embedded_hal;

use profont::{PROFONT_24_POINT, PROFONT_18_POINT};

use esp_println::println;
use esp_backtrace as _;

/* Debouncing algorythm */
pub enum Event {
    Pressed,
    Released,
    Nothing,
}
pub struct Button<T> {
    button: T,
    pressed: bool,
}

impl<T: ::embedded_hal::digital::v2::InputPin<Error = core::convert::Infallible>> Button<T> {
    pub fn new(button: T) -> Self {
        Button {
            button,
            pressed: true,
        }
    }
    pub fn check(&mut self){
        self.pressed = !self.button.is_low().unwrap();
    }

    pub fn poll(&mut self, delay :&mut Delay) -> Event {
        let pressed_now = !self.button.is_low().unwrap();
        if !self.pressed  &&  pressed_now
        {
            delay.delay_ms(30 as u32);
            self.check();
            if !self.button.is_low().unwrap() {
                Event::Pressed
            }
            else {
                Event::Nothing
            }
        }
        else if self.pressed && !pressed_now{
            delay.delay_ms(30 as u32);
            self.check();
            if self.button.is_low().unwrap()
            {
                Event::Released
            }
            else {
                Event::Nothing
            }
        }
        else{
            Event::Nothing
        }
        
    }
}

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();

    #[cfg(any(feature = "esp32"))]
    let mut system = peripherals.DPORT.split();
    #[cfg(any(feature = "esp32s2", feature = "esp32s3", feature = "esp32c3"))]
    let mut system = peripherals.SYSTEM.split();

    let mut clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    let timer_group0 = TimerGroup::new(peripherals.TIMG0, &clocks);
    let mut wdt0 = timer_group0.wdt;
    let timer_group1 = TimerGroup::new(peripherals.TIMG1, &clocks);
    let mut wdt1 = timer_group1.wdt;
    
    rtc.rwdt.disable();
    wdt0.disable();
    wdt1.disable();

    println!("About to initialize the SPI LED driver ILI9341");
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    
    /* Set corresponding pins */
    let mosi = io.pins.gpio7;
    let cs = io.pins.gpio2;
    let rst = io.pins.gpio10;
    let dc = io.pins.gpio3;
    let sck = io.pins.gpio6;
    let miso = io.pins.gpio9;
    let backlight = io.pins.gpio4;

    /* Then set backlight (set_low() - display lights up when signal is in 0, set_high() - opposite case(for example.)) */
    let mut backlight = backlight.into_push_pull_output();
    //backlight.set_low().unwrap();

    /* Configure SPI */
    let spi = spi::Spi::new(
        peripherals.SPI2,
        sck,
        mosi,
        miso,
        cs,
        80u32.MHz(),
        spi::SpiMode::Mode0,
        &mut system.peripheral_clock_control,
        &mut clocks,
    );

    let di = SPIInterfaceNoCS::new(spi, dc.into_push_pull_output());
    let reset = rst.into_push_pull_output();
    let mut delay = Delay::new(&clocks);


    let mut display = mipidsi::Builder::ili9341_rgb565(di)
        .with_display_size(240 as u16, 320 as u16)
        .with_framebuffer_size(240 as u16, 320 as u16)
        .with_orientation(Orientation::LandscapeInverted(true))
        .init(&mut delay, Some(reset))
        .unwrap();
        
    println!("Initialized");

    display.clear(Rgb565::WHITE);

    let eye_plate_tab = display.bounding_box().center() - Size::new(80, 30);
    let lollipop_plate_tab = display.bounding_box().center() - Size::new(80,0);
    let garden_plate_tab = display.bounding_box().center() + Size::new(0, 30) - Size::new(80, 0);

    let pointer_offset = Size::new(15, 10);



     
    let mut button_up = Button::new(io.pins.gpio0.into_pull_up_input());
    let mut button_down  = Button::new(io.pins.gpio1.into_pull_up_input());
    let mut button_ok = Button::new(io.pins.gpio8.into_pull_up_input());

 
    Text::new("Eye",
            eye_plate_tab,
            MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
    )
    .draw(&mut display)
    .unwrap();

    Text::new("Lollipop Guy",
            lollipop_plate_tab, //- Size::new(0, 15), 
            MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),           
    )
    .draw(&mut display)
    .unwrap();

    Text::new("Garden",
            garden_plate_tab, 
            MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
    )
    .draw(&mut display)
    .unwrap();

    Circle::new(eye_plate_tab - pointer_offset, 10)
        .into_styled(
                    PrimitiveStyleBuilder::new()
                        .stroke_color(Rgb565::BLACK)
                        .stroke_width(1)
                        .fill_color(Rgb565::BLACK)
                        .build(),
        )
        .draw(&mut display)
        .unwrap();

     


    let mut pointer_position : u8 = 1;
    let mut last_pointer_position : u8 = 1;

    loop {

        if last_pointer_position != pointer_position
        { 
            Rectangle::new( match last_pointer_position {
                                        1 => eye_plate_tab,
                                        2 => lollipop_plate_tab,
                                        3 => garden_plate_tab,
                                        _ => Point::new(0,0), 
                                    } - Size::new(17,12), Size::new(15, 15))
            .into_styled(
                PrimitiveStyleBuilder::new()
                    .fill_color(Rgb565::WHITE)
                    .build(),
            )
            .draw(&mut display)
            .unwrap();

            Circle::new(match pointer_position {
                                        1 => eye_plate_tab,
                                        2 => lollipop_plate_tab,
                                        3 => garden_plate_tab,
                                        _ => Point::new(0,0), 
                                } - pointer_offset, 10)
            .into_styled(
                        PrimitiveStyleBuilder::new()
                            .stroke_color(Rgb565::BLACK)
                            .stroke_width(1)
                            .fill_color(Rgb565::BLACK)
                            .build(),
            )
            .draw(&mut display)
            .unwrap();
             

            last_pointer_position = pointer_position
        }

        if let Event::Pressed = button_up.poll(&mut delay)
        {
            println!("pressed up");
            if pointer_position == 1 { pointer_position = 3; }
            else{ pointer_position -= 1; }
        }
        if let Event::Pressed = button_down.poll(&mut delay)
        {
            println!("pressed down");
            if pointer_position == 3{ pointer_position = 1; }
            else{ pointer_position += 1;}
        }


        if let Event::Pressed = button_ok.poll(&mut delay)
        {
            display.clear(Rgb565::WHITE);
             
            if pointer_position == 1
            {
                let default_style = MonoTextStyleBuilder::new()
                    .font(&FONT_10X20)
                    .text_color(RgbColor::BLACK)
                    .build();

                let mut vt;
                let mut x;
                let mut y;
                for i in 0..13200 {
                    vt = i as f64 / (20.0 * PI as f64);
                    if i < 8000 {
                        x = (vt - 50.0) * sin(vt);
                    } else {
                        x = (vt + 20.0) * sin(vt);
                    }
                    y = (vt - 50.0) * cos(vt);
                    if i < 8000 {
                        Text::with_alignment("'", Point::new((x + 160.0) as i32, (y + 125.0) as i32), default_style,  Alignment::Center)
                            .draw(&mut display)
                            .unwrap();
                    } else {
                        Text::with_alignment("|", Point::new((x + 160.0) as i32, (y + 125.0) as i32), default_style,  Alignment::Center)
                            .draw(&mut display)
                            .unwrap();
                    }
                }
                 
                loop
                {
                    if let Event::Pressed = button_ok.poll(&mut delay) {break;}
                }
                display.clear(Rgb565::WHITE);
                 

                Text::new("Eye",
                        eye_plate_tab,
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
                )
                .draw(&mut display)
                .unwrap();

                Text::new("Lollipop Guy",
                        lollipop_plate_tab, //- Size::new(0, 15), 
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),           
                )
                .draw(&mut display)
                .unwrap();

                Text::new("Garden",
                        garden_plate_tab, 
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
                )
                .draw(&mut display)
                .unwrap();

                Circle::new(eye_plate_tab - pointer_offset, 10)
                    .into_styled(
                                PrimitiveStyleBuilder::new()
                                    .stroke_color(Rgb565::BLACK)
                                    .stroke_width(1)
                                    .fill_color(Rgb565::BLACK)
                                    .build(),
                    )
                    .draw(&mut display)
                    .unwrap();
                 


                pointer_position = 1;
                last_pointer_position = 1;
            }
            else if pointer_position == 2
            {
                let default_style = MonoTextStyleBuilder::new()
                    .font(&FONT_10X20)
                    .text_color(RgbColor::BLACK)
                    .build();

                let mut vt;
                let mut x;
                let mut y;

                //body
                for i in 0..7000 {
                    vt = i as f64 / (40.0 * PI as f64);
                    x = (vt - 50.0) * sin(vt);
                    y = (vt + 50.0) *  cos(vt);
                    if i < 6500 || i > 6900 {
                        Text::with_alignment("'", Point::new((x + 220.0) as i32, (y + 200.0) as i32), default_style,  Alignment::Center)
                            .draw(&mut display)
                            .unwrap();
                    }
                    
                }
                 

                //head
                for i in 0..7000 {
                    vt = i as f64 / (60.0 * PI as f64);
                    x = (vt + 50.0) * cos(vt);
                    y = (vt -  50.0) * sin(vt);
                    
                    Text::with_alignment("'", Point::new((x + 220.0) as i32, (y + 60.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();

                }
                 

                //eyes
                for i in 0..1300 {
                    vt = i as f64 / (20.0 * PI as f64);
                    x = (vt - 15.0) * sin(vt);
                    y = (vt -  15.0) * cos(vt);
                    
                    Text::with_alignment("'", Point::new((x + 200.0) as i32, (y + 60.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                for i in 0..1300 {
                    vt = i as f64 / (20.0 * PI as f64);
                    x = (vt - 15.0) * sin(vt);
                    y = (vt -  15.0) * cos(vt);
                    
                    Text::with_alignment("'", Point::new((x + 240.0) as i32, (y + 60.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                //hand

                let mut b;
                for a in (125..175).rev() {
                    b = a;
                    Text::with_alignment("-", Point::new(a, b), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                //lollipop

                for i in 0..3300 {
                    vt = i as f64 / (30.0 * PI as f64);
                    x = (vt - 30.0) * sin(vt);
                    y = (vt -  30.0) * cos(vt);
                    
                    Text::with_alignment("'", Point::new((x + 110.0) as i32, (y + 110.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                loop
                {
                    if let Event::Pressed = button_ok.poll(&mut delay) {break;}
                }
                display.clear(Rgb565::WHITE);
                 
                Text::new("Eye",
                        eye_plate_tab,
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
                )
                .draw(&mut display)
                .unwrap();

                Text::new("Lollipop Guy",
                        lollipop_plate_tab, //- Size::new(0, 15), 
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),           
                )
                .draw(&mut display)
                .unwrap();

                Text::new("Garden",
                        garden_plate_tab, 
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
                )
                .draw(&mut display)
                .unwrap();

                Circle::new(eye_plate_tab - pointer_offset, 10)
                    .into_styled(
                                PrimitiveStyleBuilder::new()
                                    .stroke_color(Rgb565::BLACK)
                                    .stroke_width(1)
                                    .fill_color(Rgb565::BLACK)
                                    .build(),
                    )
                    .draw(&mut display)
                    .unwrap();
                 

                pointer_position = 1;
                last_pointer_position = 1;
            }
            else if pointer_position == 3
            {
                let default_style = MonoTextStyleBuilder::new()
                    .font(&FONT_10X20)
                    .text_color(RgbColor::BLACK)
                    .build();

                let mut n = 6.0;
                let mut d = 71.0;    
                let mut a;
                let mut r;
                let mut x;
                let mut y;

                for t in 0..361 {
                    a = t as f64 * d * (PI as f64 / 60.0);
                    r = 30.0 * sin(n * a);
                    x = r * cos(a);
                    y = r * sin(a);

                    Text::with_alignment("o", Point::new((x + 35.0) as i32, (y + 180.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 
                let pos_x = 1;
                for pos_y in 0..60 {
                    Text::with_alignment("|", Point::new(pos_x + 34, pos_y + 180), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                n = 7.0;
                d = 19.0;
                for t in 0..700 {
                    a = t as f64 * d * (PI as f64 / 300.0);
                    r = 30.0 * sin(n * a);
                    x = r * cos(a);
                    y = r * sin(a);

                    Text::with_alignment("o", Point::new((x + 90.0) as i32, (y + 140.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                for pos_y in 0..100 {
                    Text::with_alignment("|", Point::new(pos_x + 89, pos_y + 140), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                n = 2.0;
                d = 39.0;
                for t in 0..500 {
                    a = t as f64 * d * (PI as f64 / 150.0);
                    r = 30.0 * sin(n * a);
                    x = r * cos(a);
                    y = r * sin(a);

                    Text::with_alignment("S", Point::new((x + 140.0) as i32, (y + 190.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                for pos_y in 0..50 {
                    Text::with_alignment("|", Point::new(pos_x + 139, pos_y + 190), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                n = 8.0;
                d = 27.0;
                for t in 0..1000 {
                    a = t as f64 * d * (PI as f64 / 230.0);
                    r = 30.0 * sin(n * a);
                    x = r * cos(a);
                    y = r * sin(a);

                    Text::with_alignment("o", Point::new((x + 243.0) as i32, (y + 200.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 
                for pos_y in 0..85 {
                    Text::with_alignment("|", Point::new(pos_x + 242, pos_y + 200), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                n = 5.0;
                d = 97.0;
                for t in 0..700 {
                    a = t as f64 * d * (PI as f64 / 150.0);
                    r = 30.0 * sin(n * a);
                    x = r * cos(a);
                    y = r * sin(a);

                    Text::with_alignment("o", Point::new((x + 290.0) as i32, (y + 155.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 
                for pos_y in 0..85 {
                    Text::with_alignment("|", Point::new(pos_x + 289, pos_y + 155), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                n = 6.0;
                d = 71.0;
                for t in 0..2500 {
                    a = t as f64 * d * (PI as f64 / 1200.0);
                    r = 80.0 * sin(n * a);
                    x = r * cos(a);
                    y = r * sin(a);

                    Text::with_alignment("o", Point::new((x + 200.0) as i32, (y + 90.0) as i32), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                for pos_y in 0..100 {
                    Text::with_alignment("|", Point::new(pos_x + 199, pos_y + 140), default_style,  Alignment::Center)
                        .draw(&mut display)
                        .unwrap();
                }
                 

                loop
                {
                    if let Event::Pressed = button_ok.poll(&mut delay) {break;}
                }
                display.clear(Rgb565::WHITE);
                 
                Text::new("Eye",
                        eye_plate_tab,
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
                )
                .draw(&mut display)
                .unwrap();

                Text::new("Lollipop Guy",
                        lollipop_plate_tab, //- Size::new(0, 15), 
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),           
                )
                .draw(&mut display)
                .unwrap();

                Text::new("Garden",
                        garden_plate_tab, 
                        MonoTextStyle::new(&PROFONT_18_POINT, Rgb565::BLACK),
                )
                .draw(&mut display)
                .unwrap();

                Circle::new(eye_plate_tab - pointer_offset, 10)
                    .into_styled(
                                PrimitiveStyleBuilder::new()
                                    .stroke_color(Rgb565::BLACK)
                                    .stroke_width(1)
                                    .fill_color(Rgb565::BLACK)
                                    .build(),
                    )
                    .draw(&mut display)
                    .unwrap();
                 

                pointer_position = 1;
                last_pointer_position = 1;
            }
        }
    }
}