use appcui::graphics::{CharAttribute, CharFlags, Character, Color};

use super::{AnsiErrorKind, AnsiParserBuilder, OnError};

fn parse(text: &str) -> Vec<Character> {
    let mut output = Vec::new();
    AnsiParserBuilder::new()
        .build()
        .write_into_vec(text, &mut output);
    output
}

fn character(code: char, foreground: Color, background: Color, flags: CharFlags) -> Character {
    Character::new(code, foreground, background, flags)
}

#[test]
fn plain_text_uses_the_default_style() {
    let output = parse("Hi");
    assert_eq!(
        output,
        vec![
            character('H', Color::White, Color::Black, CharFlags::None),
            character('i', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn keeps_newlines_tabs_and_carriage_returns() {
    let output = parse("A\n\tB\r");
    assert_eq!(
        output.iter().map(|ch| ch.code).collect::<Vec<_>>(),
        vec!['A', '\n', '\t', 'B', '\r']
    );
}

#[test]
fn appends_without_clearing_the_vector() {
    let mut output = vec![character('Z', Color::Green, Color::Black, CharFlags::None)];
    AnsiParserBuilder::new()
        .build()
        .write_into_vec("A", &mut output);
    assert_eq!(output[0].code, 'Z');
    assert_eq!(
        output[1],
        character('A', Color::White, Color::Black, CharFlags::None)
    );
}

#[test]
fn applies_standard_and_bright_colors() {
    assert_eq!(
        parse("\x1b[31mR\x1b[44mB"),
        vec![
            character('R', Color::DarkRed, Color::Black, CharFlags::None),
            character('B', Color::DarkRed, Color::DarkBlue, CharFlags::None),
        ]
    );
    assert_eq!(
        parse("\x1b[91;103mX"),
        vec![character('X', Color::Red, Color::Yellow, CharFlags::None)]
    );
}

#[test]
fn reset_and_default_colors_restore_white_on_black() {
    let output = parse("\x1b[1;31;44mA\x1b[0mB\x1b[32mC\x1b[39;49mD\x1b[1mE\x1b[mF");
    assert_eq!(
        output,
        vec![
            character('A', Color::DarkRed, Color::DarkBlue, CharFlags::Bold),
            character('B', Color::White, Color::Black, CharFlags::None),
            character('C', Color::DarkGreen, Color::Black, CharFlags::None),
            character('D', Color::White, Color::Black, CharFlags::None),
            character('E', Color::White, Color::Black, CharFlags::Bold),
            character('F', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn applies_and_clears_style_flags() {
    let styled =
        CharFlags::Bold | CharFlags::Italic | CharFlags::Underline | CharFlags::StrikeThrough;
    assert_eq!(
        parse("\x1b[1;3;4;9mA\x1b[23mB\x1b[22;29;24mC"),
        vec![
            character('A', Color::White, Color::Black, styled),
            character(
                'B',
                Color::White,
                Color::Black,
                CharFlags::Bold | CharFlags::Underline | CharFlags::StrikeThrough
            ),
            character('C', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn underline_styles_replace_each_other() {
    assert_eq!(
        parse("\x1b[4mA\x1b[21mB\x1b[4:3mC\x1b[4:4mD\x1b[24mE"),
        vec![
            character('A', Color::White, Color::Black, CharFlags::Underline),
            character('B', Color::White, Color::Black, CharFlags::DoubleUnderline),
            character('C', Color::White, Color::Black, CharFlags::CurlyUnderline),
            character('D', Color::White, Color::Black, CharFlags::DottedUnderline),
            character('E', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn reverse_video_swaps_colors_until_it_is_turned_off() {
    assert_eq!(
        parse("\x1b[31;44m\x1b[7mA\x1b[27mB\x1b[7m\x1b[0mC"),
        vec![
            character('A', Color::DarkBlue, Color::DarkRed, CharFlags::None),
            character('B', Color::DarkRed, Color::DarkBlue, CharFlags::None),
            character('C', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn reads_indexed_and_truecolor_sequences() {
    let orange = Color::from_rgb(255, 128, 0);
    let blue = Color::from_rgb(0, 128, 255);
    assert_eq!(
        parse("\x1b[38;5;1mA\x1b[48;5;9mB\x1b[38;2;255;128;0mC\x1b[48;2;0;128;255mD"),
        vec![
            character('A', Color::DarkRed, Color::Black, CharFlags::None),
            character('B', Color::DarkRed, Color::Red, CharFlags::None),
            character('C', orange, Color::Red, CharFlags::None),
            character('D', orange, blue, CharFlags::None),
        ]
    );
    assert_eq!(
        parse("\x1b[38:2:255:128:0mA\x1b[38:2::0:128:255mB\x1b[38:5:196mC"),
        vec![
            character('A', orange, Color::Black, CharFlags::None),
            character('B', blue, Color::Black, CharFlags::None),
            character(
                'C',
                Color::from_rgb(255, 0, 0),
                Color::Black,
                CharFlags::None
            ),
        ]
    );
}

#[test]
fn ignores_cursor_and_screen_sequences_but_keeps_the_style() {
    let output = parse("\x1b[31mA\x1b[2J\x1b[10;20H\x1b[AB\x1b[?25lC\x1b7D");
    assert_eq!(
        output,
        vec![
            character('A', Color::DarkRed, Color::Black, CharFlags::None),
            character('B', Color::DarkRed, Color::Black, CharFlags::None),
            character('C', Color::DarkRed, Color::Black, CharFlags::None),
            character('D', Color::DarkRed, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn ignores_operating_system_commands_and_bare_escapes() {
    assert_eq!(
        parse("\x1b]0;title\x07Hi\x1b]0;title\x1b\\!\x1bc?"),
        vec![
            character('H', Color::White, Color::Black, CharFlags::None),
            character('i', Color::White, Color::Black, CharFlags::None),
            character('!', Color::White, Color::Black, CharFlags::None),
            character('?', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn unsupported_sgr_attributes_leave_text_and_style_unchanged() {
    assert_eq!(
        parse("\x1b[1m\x1b[2;5;8mA"),
        vec![character('A', Color::White, Color::Black, CharFlags::Bold)]
    );
}

#[test]
fn drops_incomplete_sequences_without_emitting_them() {
    assert_eq!(
        parse("Go\x1b[31\x1bé"),
        vec![
            character('G', Color::White, Color::Black, CharFlags::None),
            character('o', Color::White, Color::Black, CharFlags::None),
            character('é', Color::White, Color::Black, CharFlags::None),
        ]
    );
}

#[test]
fn reset_returns_to_the_parser_default_attribute() {
    let mut parser = AnsiParserBuilder::new()
        .default_attr(CharAttribute::new(
            Color::Green,
            Color::DarkBlue,
            CharFlags::Italic,
        ))
        .build();
    let mut output = Vec::new();
    parser.write_into_vec("A\x1b[31mB\x1b[0mC\x1b[39;49mD", &mut output);
    assert!(parser.errors().is_empty());
    assert_eq!(
        output,
        vec![
            character('A', Color::Green, Color::DarkBlue, CharFlags::Italic),
            character('B', Color::DarkRed, Color::DarkBlue, CharFlags::Italic),
            character('C', Color::Green, Color::DarkBlue, CharFlags::Italic),
            character('D', Color::Green, Color::DarkBlue, CharFlags::Italic),
        ]
    );
}

#[test]
fn ignore_drops_malformed_sequences_without_recording_them() {
    let mut parser = AnsiParserBuilder::new().build();
    let mut output = Vec::new();
    parser.write_into_vec("A\x1b[31€B", &mut output);
    assert!(parser.errors().is_empty());
    assert_eq!(codes_of(&output), vec!['A', '€', 'B']);
}

#[test]
fn collect_records_malformed_sequences_and_continues() {
    let mut parser = AnsiParserBuilder::new().on_error(OnError::Collect).build();
    let mut output = Vec::new();
    let text = "\x1b[31€\x1b[32mZ\x1b";
    parser.write_into_vec(text, &mut output);
    assert_eq!(codes_of(&output), vec!['€', 'Z']);
    assert_eq!(output[1].foreground, Color::DarkGreen);
    assert_eq!(
        parser.errors(),
        &[
            AnsiErrorKind::InvalidCsi('€').at(0),
            AnsiErrorKind::TruncatedEscape.at(text.rfind('\u{1b}').unwrap()),
        ]
    );
}

#[test]
fn collect_reports_a_truncated_string_and_ignores_finished_commands() {
    let mut parser = AnsiParserBuilder::new().on_error(OnError::Collect).build();
    let mut output = Vec::new();
    let text = "\x1b[2J\x1b]0;title\x07X\x1b]unterminated";
    parser.write_into_vec(text, &mut output);
    assert_eq!(codes_of(&output), vec!['X']);
    let truncated_at = text.find('X').unwrap() + 1;
    assert_eq!(
        parser.errors(),
        &[AnsiErrorKind::TruncatedString.at(truncated_at)]
    );
}

#[test]
fn stop_keeps_characters_written_before_the_first_error() {
    let mut parser = AnsiParserBuilder::new().on_error(OnError::Stop).build();
    let mut output = Vec::new();
    parser.write_into_vec("AB\x1b[31€CD", &mut output);
    assert_eq!(codes_of(&output), vec!['A', 'B']);
    assert_eq!(parser.errors(), &[AnsiErrorKind::InvalidCsi('€').at(2)]);
}

#[test]
fn the_next_write_clears_previous_errors() {
    let mut parser = AnsiParserBuilder::new().on_error(OnError::Collect).build();
    parser.write_into_vec("\x1b", &mut Vec::new());
    assert_eq!(parser.errors(), &[AnsiErrorKind::TruncatedEscape.at(0)]);
    parser.write_into_vec("Hi", &mut Vec::new());
    assert!(parser.errors().is_empty());
}

fn codes_of(chars: &[Character]) -> Vec<char> {
    chars.iter().map(|ch| ch.code).collect()
}

impl AnsiErrorKind {
    fn at(self, offset: usize) -> super::AnsiError {
        super::AnsiError { offset, kind: self }
    }
}
