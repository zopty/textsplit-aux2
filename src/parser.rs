use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::char,
    combinator::map,
    multi::fold_many0,
    sequence::delimited,
};
use serde::Serialize;
use serde_json;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct TextElement {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    pub text: String,
}

impl TextElement {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self)
    }
}

#[derive(Clone, Debug, Default)]
struct Style {
    size: Option<f32>,
    font: Option<String>,
    is_bold: Option<bool>,
    is_italic: Option<bool>,
    color: Option<String>,
}

enum Action<'a> {
    UpdateStyle(
        (
            Option<Option<f32>>,
            Option<Option<String>>,
            Option<Option<(bool, bool)>>,
        ),
    ),
    ResetStyle,
    UpdateColor(String),
    ResetColor,
    AppendText(&'a str),
}

fn parse_optional_param(
    input: &str,
) -> IResult<
    &str,
    (
        Option<Option<f32>>,
        Option<Option<String>>,
        Option<Option<(bool, bool)>>,
    ),
> {
    let (input, content) = delimited(char('<'), take_until(">"), char('>')).parse(input)?;
    if !content.starts_with('s') || content == "s" {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Verify,
        )));
    }

    let parts: Vec<&str> = content.split(',').collect();

    let size = parts
        .get(0)
        .and_then(|s| s.get(1..))
        .map(|s| s.parse::<f32>().ok());

    let font = parts.get(1).map(|s| {
        if s.is_empty() {
            None
        } else {
            Some(s.to_string())
        }
    });

    let flags = parts.get(2).map(|s| {
        if s.is_empty() {
            None
        } else {
            Some((s.contains('B'), s.contains('I')))
        }
    });

    Ok((input, (size, font, flags)))
}

fn parse_color(input: &str) -> IResult<&str, String> {
    map(
        delimited(
            tag("<#"),
            take_while1(|c: char| c.is_ascii_hexdigit()),
            char('>'),
        ),
        |s: &str| s.to_string(),
    )
    .parse(input)
}

fn parse_text_until_tag(input: &str) -> IResult<&str, &str> {
    if input.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Eof,
        )));
    }
    let s_pos = input.find("<s").unwrap_or(input.len());
    let color_open_pos = input
        .find("<#")
        .map(|i| {
            if input[i..].starts_with("<#>") {
                input.len()
            } else {
                i
            }
        })
        .unwrap_or(input.len());
    let color_close_pos = input.find("<#>").unwrap_or(input.len());

    let pos = s_pos.min(color_open_pos).min(color_close_pos);
    let (text, rest) = input.split_at(pos);
    Ok((rest, text))
}

fn parse_action(input: &'_ str) -> IResult<&'_ str, Action<'_>> {
    alt((
        map(parse_optional_param, Action::UpdateStyle),
        map(tag("<s>"), |_| Action::ResetStyle),
        map(parse_color, Action::UpdateColor),
        map(tag("<#>"), |_| Action::ResetColor),
        map(parse_text_until_tag, |s| Action::AppendText(s)),
    ))
    .parse(input)
}

pub fn parse_markup(input: &str) -> Result<Vec<TextElement>, String> {
    let (rem, (elements, _)) = fold_many0(
        parse_action,
        || (Vec::<TextElement>::new(), Style::default()),
        |(mut elements, mut style), action| {
            match action {
                Action::UpdateStyle((size, font, flags)) => {
                    if let Some(s) = size {
                        style.size = s;
                    }
                    if let Some(f) = font {
                        style.font = f;
                    }
                    if let Some(fl) = flags {
                        if let Some((b, i)) = fl {
                            style.is_bold = Some(b);
                            style.is_italic = Some(i);
                        } else {
                            style.is_bold = None;
                            style.is_italic = None;
                        }
                    }
                }
                Action::ResetStyle => {
                    style.size = None;
                    style.font = None;
                    style.is_bold = None;
                    style.is_italic = None;
                }
                Action::UpdateColor(color) => {
                    style.color = Some(color);
                }
                Action::ResetColor => {
                    style.color = None;
                }
                Action::AppendText(text) => {
                    if !text.is_empty() {
                        elements.push(TextElement {
                            size: style.size,
                            font: style.font.clone(),
                            is_bold: style.is_bold,
                            is_italic: style.is_italic,
                            color: style.color.clone(),
                            text: text.to_string(),
                        });
                    }
                }
            }
            (elements, style)
        },
    )
    .parse(input)
    .map_err(|e| e.to_string())?;

    if rem.is_empty() {
        Ok(elements)
    } else {
        Err(format!("Unparsed input remaining: {}", rem))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_color_reset() {
        let input = "<#ff0000>red text<#>default text";
        let result = parse_markup(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "red text");
        assert_eq!(result[0].color, Some("ff0000".to_string()));
        assert_eq!(result[1].text, "default text");
        assert_eq!(result[1].color, None);
    }

    #[test]
    fn test_style_carryover_after_color_reset() {
        let input = "<s12,,I><#ff0000>italic and red<#>italic only";
        let result = parse_markup(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "italic and red");
        assert_eq!(result[0].size, Some(12.0));
        assert_eq!(result[0].is_italic, Some(true));
        assert_eq!(result[0].color, Some("ff0000".to_string()));
        assert_eq!(result[1].text, "italic only");
        assert_eq!(result[1].size, Some(12.0));
        assert_eq!(result[1].is_italic, Some(true));
        assert_eq!(result[1].color, None);
    }

    #[test]
    fn test_treat_unrecognized_tags_as_text() {
        let input = "Initial Text<other>Tag<s12.5,,><#FF0000>Hello World!";
        let result = parse_markup(input);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let elements = result.unwrap();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].text, "Initial Text<other>Tag");
        assert_eq!(elements[1].text, "Hello World!");
        assert_eq!(elements[1].size, Some(12.5));
        assert_eq!(elements[1].color, Some("FF0000".to_string()));
    }

    #[test]
    fn test_color_only_element() {
        let input = "Before<#123456>Colored Text";
        let result = parse_markup(input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].text, "Before");
        assert_eq!(result[1].color, Some("123456".to_string()));
        assert_eq!(result[1].text, "Colored Text");
    }

    #[test]
    fn test_no_trailing_text() {
        let input = "<s10,Comic Sans MS,B><#123456>";
        let result = parse_markup(input).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_consecutive_tags() {
        let input = "<s10><#ff0000>text";
        let result = parse_markup(input).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].size, Some(10.0));
        assert_eq!(result[0].color, Some("ff0000".to_string()));
        assert_eq!(result[0].text, "text");
    }

    #[test]
    fn test_style_reset() {
        let input = "<s50,Arial,B><#123>bold, 50px, red<s>normal text";
        let result = parse_markup(input).unwrap();
        assert_eq!(result.len(), 2);

        let el1 = &result[0];
        assert_eq!(el1.text, "bold, 50px, red");
        assert_eq!(el1.size, Some(50.0));
        assert_eq!(el1.font, Some("Arial".to_string()));
        assert_eq!(el1.is_bold, Some(true));
        assert_eq!(el1.color, Some("123".to_string()));

        let el2 = &result[1];
        assert_eq!(el2.text, "normal text");
        assert_eq!(el2.size, None);
        assert_eq!(el2.font, None);
        assert_eq!(el2.is_bold, None);
        assert_eq!(el2.color, Some("123".to_string())); // color is preserved
    }

    #[test]
    fn test_empty_size_resets_size() {
        let input = "<s50>size 50<s,Arial>size reset, font Arial<s>back to default";
        let result = parse_markup(input).unwrap();
        assert_eq!(result.len(), 3);

        assert_eq!(result[0].size, Some(50.0));
        assert_eq!(result[0].font, None);

        assert_eq!(result[1].size, None);
        assert_eq!(result[1].font, Some("Arial".to_string()));

        assert_eq!(result[2].size, None);
        assert_eq!(result[2].font, None);
    }
}
