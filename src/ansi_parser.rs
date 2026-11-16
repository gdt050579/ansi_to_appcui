use appcui::graphics::*;
use crate::attr_utils::{self, *};

pub struct AnsiParser {
    pub(crate) default_attr: CharAttribute,
    current_attr: CharAttribute,
    reverse_flag: bool,
    sgr_values: Vec<u16>,
    /// `sgr_sub_starts[i]` is where parameter `i`'s colon-subparameters begin in `sgr_subs`.
    sgr_sub_starts: Vec<usize>,
    sgr_subs: Vec<Option<u16>>,
}

impl AnsiParser {
    /// Appends the text of an ANSI string to `vec`, one [`Character`] per Unicode scalar.
    ///
    /// The style starts as this parser's default attribute. SGR reset (`0`) restores that
    /// attribute, and `39` / `49` restore its foreground and background. Other SGR sequences
    /// update the colors and style of later characters: the 16 ANSI colors, 256-color and
    /// 24-bit colors, bold, italic, underline (single, double, curly, and dotted),
    /// strike-through, and reverse video. 24-bit and 256-color values go through
    /// [`Color::from_rgb`], so they follow AppCUI's `TRUE_COLORS` feature.
    ///
    /// Every other escape sequence, including cursor movement, erase, and window titles,
    /// is discarded. It does not emit characters and does not change the current style.
    /// Characters already in `vec` stay in place.
    pub fn write_into_vec(&mut self, text: &str, vec: &mut Vec<Character>) {
        self.reset();
        let mut rest = text;
        while let Some(ch) = rest.chars().next() {
            let after = &rest[ch.len_utf8()..];
            match ch {
                '\u{1b}' => rest = self.parse_escape_sequance(after),
                '\u{9b}' => rest = self.parse_csi_sequance(after),
                '\u{9d}' => rest = self.skip_string(after, true),
                '\u{90}' | '\u{98}' | '\u{9e}' | '\u{9f}' => rest = self.skip_string(after, false),
                '\t' | '\n' | '\r' => {
                    self.push_char(vec, ch);
                    rest = after;
                }
                other if other.is_control() => rest = after,
                other => {
                    self.push_char(vec, other);
                    rest = after;
                }
            }
        }
    }

    fn reset(&mut self) {
        self.current_attr = self.default_attr;
        self.reverse_flag = false;
    }

    fn push_char(&self, vec: &mut Vec<Character>, code: char) {
        let (foreground, background) = if self.reverse_flag {
            (self.current_attr.background, self.current_attr.foreground)
        } else {
            (self.current_attr.foreground, self.current_attr.background)
        };
        vec.push(Character::new(
            code,
            foreground,
            background,
            self.current_attr.flags,
        ));
    }

    fn parse_escape_sequance<'a>(&mut self, input: &'a str) -> &'a str {
        let mut chars = input.chars();
        let Some(next) = chars.next() else {
            return "";
        };
        match next {
            '[' => self.parse_csi_sequance(chars.as_str()),
            ']' => self.skip_string(chars.as_str(), true),
            'P' | 'X' | '^' | '_' => self.skip_string(chars.as_str(), false),
            '\u{20}'..='\u{2f}' => self.skip_esc_final(chars.as_str()),
            '\u{30}'..='\u{7e}' => chars.as_str(),
            _ => input,
        }
    }

    fn skip_esc_final<'a>(&self, input: &'a str) -> &'a str {
        let mut rest = input;
        while let Some(ch) = rest.chars().next() {
            if ('\u{20}'..='\u{2f}').contains(&ch) {
                rest = &rest[ch.len_utf8()..];
                continue;
            }
            if ('\u{30}'..='\u{7e}').contains(&ch) {
                return &rest[ch.len_utf8()..];
            }
            return rest;
        }
        ""
    }

    fn skip_string<'a>(&self, input: &'a str, allow_bel: bool) -> &'a str {
        let mut rest = input;
        while let Some(ch) = rest.chars().next() {
            let after = &rest[ch.len_utf8()..];
            if (allow_bel && ch == '\u{7}') || ch == '\u{18}' || ch == '\u{1a}' {
                return after;
            }
            if ch == '\u{1b}' {
                if let Some(stripped) = after.strip_prefix('\\') {
                    return stripped;
                }
                return rest;
            }
            rest = after;
        }
        ""
    }

    fn parse_csi_sequance<'a>(&mut self, input: &'a str) -> &'a str {
        self.sgr_values.clear();
        self.sgr_sub_starts.clear();
        self.sgr_subs.clear();
        let mut rest = input;
        let mut value: Option<u16> = None;
        let mut sub_start = 0;
        let mut saw_param = false;
        let mut in_sub = false;
        let mut private = false;
        let mut intermediate = false;

        loop {
            let Some(ch) = rest.chars().next() else {
                return "";
            };
            let after = &rest[ch.len_utf8()..];
            match ch {
                '0'..='9' => {
                    let digit = u16::from(ch as u8 - b'0');
                    if in_sub {
                        if let Some(slot) = self.sgr_subs.last_mut() {
                            *slot =
                                Some(slot.unwrap_or(0).saturating_mul(10).saturating_add(digit));
                        }
                    } else {
                        value = Some(value.unwrap_or(0).saturating_mul(10).saturating_add(digit));
                    }
                    saw_param = true;
                    rest = after;
                }
                ':' => {
                    self.sgr_subs.push(None);
                    in_sub = true;
                    saw_param = true;
                    rest = after;
                }
                ';' => {
                    self.push_sgr_value(value, sub_start);
                    value = None;
                    sub_start = self.sgr_subs.len();
                    in_sub = false;
                    saw_param = true;
                    rest = after;
                }
                '?' | '<' | '=' | '>' => {
                    private = true;
                    saw_param = true;
                    rest = after;
                }
                '\u{20}'..='\u{2f}' => {
                    intermediate = true;
                    rest = after;
                }
                '\u{40}'..='\u{7e}' => {
                    if saw_param {
                        self.push_sgr_value(value, sub_start);
                    }
                    if ch == 'm' && !private && !intermediate {
                        self.apply_sgr();
                    }
                    return after;
                }
                _ => return rest,
            }
        }
    }

    fn push_sgr_value(&mut self, value: Option<u16>, sub_start: usize) {
        self.sgr_values.push(value.unwrap_or(0));
        self.sgr_sub_starts.push(sub_start);
    }

    fn param_subs(&self, index: usize) -> &[Option<u16>] {
        let start = self.sgr_sub_starts[index];
        let end = self
            .sgr_sub_starts
            .get(index + 1)
            .copied()
            .unwrap_or(self.sgr_subs.len());
        &self.sgr_subs[start..end]
    }

    fn apply_sgr(&mut self) {
        if self.sgr_values.is_empty() {
            self.reset();
            return;
        }

        let mut index = 0;
        while index < self.sgr_values.len() {
            let code = self.sgr_values[index];
            let mut consumed = 1;
            match code {
                0 => self.reset(),
                1 => self.current_attr.flags.set(CharFlags::Bold),
                3 => self.current_attr.flags.set(CharFlags::Italic),
                4 => {
                    let underline = self
                        .param_subs(index)
                        .first()
                        .copied()
                        .flatten()
                        .unwrap_or(1);
                    set_underline(&mut self.current_attr.flags, underline);
                }
                7 => self.reverse_flag = true,
                9 => self.current_attr.flags.set(CharFlags::StrikeThrough),
                // in AppCUI folosesc 21 (underline) si 22 clear bold.
                21 => set_underline(&mut self.current_attr.flags, 2),
                22 => self.current_attr.flags.remove(CharFlags::Bold),
                23 => self.current_attr.flags.remove(CharFlags::Italic),
                24 => set_underline(&mut self.current_attr.flags, 0),
                27 => self.reverse_flag = false,
                29 => self.current_attr.flags.remove(CharFlags::StrikeThrough),
                30..=37 => self.current_attr.foreground = ANSI_NORMAL[(code - 30) as usize],
                38 | 48 | 58 => {
                    let (color, count) = self.extended_color(index);
                    consumed = count;
                    if code == 38 {
                        if let Some(color) = color {
                            self.current_attr.foreground = color;
                        }
                    } else if code == 48 {
                        if let Some(color) = color {
                            self.current_attr.background = color;
                        }
                    }
                }
                39 => self.current_attr.foreground = self.default_attr.foreground,
                40..=47 => self.current_attr.background = ANSI_NORMAL[(code - 40) as usize],
                49 => self.current_attr.background = self.default_attr.background,
                90..=97 => self.current_attr.foreground = ANSI_BRIGHT[(code - 90) as usize],
                100..=107 => self.current_attr.background = ANSI_BRIGHT[(code - 100) as usize],
                _ => {}
            }
            index += consumed;
        }
    }
    fn extended_color(&self, index: usize) -> (Option<Color>, usize) {
        let subs = self.param_subs(index);
        if !subs.is_empty() {
            return (color_from_subs(subs), 1);
        }
        let params = &self.sgr_values[index..];
        if params.len() < 2 {
            return (None, 1);
        }
        match params[1] {
            5 => {
                if params.len() >= 3 {
                    (Some(color_from_index(params[2])), 3)
                } else {
                    (None, params.len())
                }
            }
            2 => {
                if params.len() >= 5 {
                    (
                        Some(attr_utils::color_from_u16_rgb(params[2], params[3], params[4])),
                        5,
                    )
                } else {
                    (None, params.len())
                }
            }
            _ => (None, 2),
        }
    }
}

pub struct AnsiParserBuilder {
    parser: AnsiParser,
}

impl AnsiParserBuilder {
    pub fn new() -> Self {
        let default_attr = CharAttribute::default();
        Self {
            parser: AnsiParser {
                default_attr,
                current_attr: default_attr,
                reverse_flag: false,
                sgr_values: Vec::new(),
                sgr_sub_starts: Vec::new(),
                sgr_subs: Vec::new(),
            },
        }
    }
    pub fn default_attr(mut self, default_attr: CharAttribute) -> Self {
        self.parser.default_attr = default_attr;
        self.parser.current_attr = default_attr;
        self
    }
    pub fn build(self) -> AnsiParser {
        self.parser
    }
}
