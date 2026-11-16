use appcui::graphics::*;

pub(crate) const ANSI_NORMAL: [Color; 8] = [
    Color::Black,
    Color::DarkRed,
    Color::DarkGreen,
    Color::Olive,
    Color::DarkBlue,
    Color::Magenta,
    Color::Teal,
    Color::Silver,
];

pub(crate) const ANSI_BRIGHT: [Color; 8] = [
    Color::Gray,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Pink,
    Color::Aqua,
    Color::White,
];

fn channel_from_u16(value: u16) -> u8 {
    value.clamp(0, 255) as u8
}

pub(crate) fn set_underline(flags: &mut CharFlags, style: u16) {
    flags.remove(
        CharFlags::Underline
            | CharFlags::DoubleUnderline
            | CharFlags::CurlyUnderline
            | CharFlags::DottedUnderline,
    );
    let selected = match style {
        0 => None,
        2 => Some(CharFlags::DoubleUnderline),
        3 => Some(CharFlags::CurlyUnderline),
        4 => Some(CharFlags::DottedUnderline),
        _ => Some(CharFlags::Underline),
    };
    if let Some(selected) = selected {
        flags.set(selected);
    }
}

pub(crate) fn color_from_subs(subs: &[Option<u16>]) -> Option<Color> {
    let mode = subs.first().copied().flatten().unwrap_or(0);
    let parts = &subs[1..];
    match mode {
        5 => Some(color_from_index(
            parts.first().copied().flatten().unwrap_or(0),
        )),
        2 => color_from_parts(parts),
        _ => None,
    }
}

fn color_from_parts(parts: &[Option<u16>]) -> Option<Color> {
    let (red, green, blue) = if parts.len() >= 4 {
        (parts[1], parts[2], parts[3])
    } else if parts.len() == 3 {
        (parts[0], parts[1], parts[2])
    } else {
        return None;
    };
    Some(Color::from_rgb(
        channel_from_u16(red.unwrap_or(0)),
        channel_from_u16(green.unwrap_or(0)),
        channel_from_u16(blue.unwrap_or(0)),
    ))
}

pub(crate) fn color_from_u16_rgb(red: u16, green: u16, blue: u16) -> Color {
    Color::from_rgb(
        channel_from_u16(red),
        channel_from_u16(green),
        channel_from_u16(blue),
    )
}

pub(crate) fn color_from_index(index: u16) -> Color {
    match (index % 256) as u8 {
        value @ 0..=7 => ANSI_NORMAL[value as usize],
        value @ 8..=15 => ANSI_BRIGHT[(value - 8) as usize],
        value @ 16..=231 => {
            let offset = (value - 16) as u16;
            let red = offset / 36;
            let green = (offset % 36) / 6;
            let blue = offset % 6;
            let r = if red == 0 {
                0
            } else {
                channel_from_u16(55 + 40 * red)
            };
            let g = if green == 0 {
                0
            } else {
                channel_from_u16(55 + 40 * green)
            };
            let b = if blue == 0 {
                0
            } else {
                channel_from_u16(55 + 40 * blue)
            };
            Color::from_rgb(r, g, b)
        }
        value => {
            let level = u16::from(value - 232);
            let gray = channel_from_u16(8 + 10 * level);
            Color::from_rgb(gray, gray, gray)
        }
    }
}
