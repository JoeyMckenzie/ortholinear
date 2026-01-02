use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Splits text into spans, bolding any @mentions (e.g., @foo.bar)
fn style_with_mentions(text: &str, base_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = text;

    while let Some(at_pos) = remaining.find('@') {
        // Add text before the @
        if at_pos > 0 {
            spans.push(Span::styled(remaining[..at_pos].to_string(), base_style));
        }

        // Find the end of the mention (alphanumeric, dots, underscores, hyphens)
        let mention_start = at_pos;
        let after_at = &remaining[at_pos + 1..];
        let mention_len = after_at
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '.' || *c == '_' || *c == '-')
            .map(|c| c.len_utf8())
            .sum::<usize>();

        if mention_len > 0 {
            // Valid mention - bold it
            let mention = &remaining[mention_start..mention_start + 1 + mention_len];
            spans.push(Span::styled(
                mention.to_string(),
                base_style.add_modifier(Modifier::BOLD).fg(Color::Cyan),
            ));
            remaining = &remaining[mention_start + 1 + mention_len..];
        } else {
            // Just a lone @ - add it as regular text
            spans.push(Span::styled("@".to_string(), base_style));
            remaining = &remaining[at_pos + 1..];
        }
    }

    // Add any remaining text
    if !remaining.is_empty() {
        spans.push(Span::styled(remaining.to_string(), base_style));
    }

    spans
}

pub fn render_markdown(text: &str) -> Vec<Line<'static>> {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(text, options);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut current_line: Vec<Span<'static>> = Vec::new();
    let mut style_stack: Vec<Style> = vec![Style::default()];
    let mut in_code_block = false;
    let mut list_depth: usize = 0;
    let mut ordered_list_index: Option<u64> = None;

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    let style = match level {
                        HeadingLevel::H1 => Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                        HeadingLevel::H2 => Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                        HeadingLevel::H3 => Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                        _ => Style::default().add_modifier(Modifier::BOLD),
                    };
                    style_stack.push(style);
                }
                Tag::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    // Add language indicator if present
                    if let CodeBlockKind::Fenced(lang) = kind {
                        if !lang.is_empty() {
                            lines.push(Line::from(Span::styled(
                                format!("┌─ {} ", lang),
                                Style::default().fg(Color::DarkGray),
                            )));
                        } else {
                            lines.push(Line::from(Span::styled(
                                "┌───",
                                Style::default().fg(Color::DarkGray),
                            )));
                        }
                    }
                    style_stack.push(Style::default().fg(Color::Green));
                }
                Tag::List(start) => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    list_depth += 1;
                    ordered_list_index = start;
                }
                Tag::Item => {
                    let indent = "  ".repeat(list_depth.saturating_sub(1));
                    let bullet = if let Some(idx) = ordered_list_index {
                        let s = format!("{}{}. ", indent, idx);
                        ordered_list_index = Some(idx + 1);
                        s
                    } else {
                        format!("{}• ", indent)
                    };
                    current_line.push(Span::styled(bullet, Style::default().fg(Color::Yellow)));
                }
                Tag::Emphasis => {
                    let current = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(current.add_modifier(Modifier::ITALIC));
                }
                Tag::Strong => {
                    let current = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(current.add_modifier(Modifier::BOLD));
                }
                Tag::Strikethrough => {
                    let current = *style_stack.last().unwrap_or(&Style::default());
                    style_stack.push(current.add_modifier(Modifier::CROSSED_OUT));
                }
                Tag::Link { .. } => {
                    style_stack.push(
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::UNDERLINED),
                    );
                }
                Tag::BlockQuote(_) => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    current_line.push(Span::styled("│ ", Style::default().fg(Color::DarkGray)));
                    style_stack.push(
                        Style::default()
                            .fg(Color::Gray)
                            .add_modifier(Modifier::ITALIC),
                    );
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    style_stack.pop();
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::Paragraph => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    lines.push(Line::from(""));
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    style_stack.pop();
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                    lines.push(Line::from(Span::styled(
                        "└───",
                        Style::default().fg(Color::DarkGray),
                    )));
                }
                TagEnd::List(_) => {
                    list_depth = list_depth.saturating_sub(1);
                    if list_depth == 0 {
                        ordered_list_index = None;
                    }
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                TagEnd::Item => {
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link => {
                    style_stack.pop();
                }
                TagEnd::BlockQuote(_) => {
                    style_stack.pop();
                    if !current_line.is_empty() {
                        lines.push(Line::from(std::mem::take(&mut current_line)));
                    }
                }
                _ => {}
            },
            Event::Text(text) => {
                let style = *style_stack.last().unwrap_or(&Style::default());
                if in_code_block {
                    // Split code block text by newlines
                    for (i, line_text) in text.lines().enumerate() {
                        if i > 0 {
                            lines.push(Line::from(std::mem::take(&mut current_line)));
                        }
                        current_line.push(Span::styled(format!("│ {}", line_text), style));
                    }
                } else {
                    // Apply @mention highlighting
                    current_line.extend(style_with_mentions(&text, style));
                }
            }
            Event::Code(code) => {
                current_line.push(Span::styled(
                    format!("`{}`", code),
                    Style::default().fg(Color::Green),
                ));
            }
            Event::SoftBreak => {
                current_line.push(Span::raw(" "));
            }
            Event::HardBreak => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
            }
            Event::Rule => {
                if !current_line.is_empty() {
                    lines.push(Line::from(std::mem::take(&mut current_line)));
                }
                lines.push(Line::from(Span::styled(
                    "────────────────────────────────",
                    Style::default().fg(Color::DarkGray),
                )));
            }
            _ => {}
        }
    }

    if !current_line.is_empty() {
        lines.push(Line::from(current_line));
    }

    while lines.last().is_some_and(|l| l.spans.is_empty()) {
        lines.pop();
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let result = render_markdown("");
        assert!(result.is_empty());
    }

    #[test]
    fn plain_text_renders_as_single_line() {
        let result = render_markdown("Hello world");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].spans.len(), 1);
        assert_eq!(result[0].spans[0].content, "Hello world");
    }

    #[test]
    fn h1_has_cyan_bold_style() {
        let result = render_markdown("# Header One");
        assert!(!result.is_empty());
        let first_line = &result[0];
        assert!(!first_line.spans.is_empty());
        let style = first_line.spans[0].style;
        assert_eq!(style.fg, Some(Color::Cyan));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn h2_has_blue_bold_style() {
        let result = render_markdown("## Header Two");
        assert!(!result.is_empty());
        let first_line = &result[0];
        assert!(!first_line.spans.is_empty());
        let style = first_line.spans[0].style;
        assert_eq!(style.fg, Some(Color::Blue));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn h3_has_magenta_bold_style() {
        let result = render_markdown("### Header Three");
        assert!(!result.is_empty());
        let first_line = &result[0];
        assert!(!first_line.spans.is_empty());
        let style = first_line.spans[0].style;
        assert_eq!(style.fg, Some(Color::Magenta));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn bold_text_has_bold_modifier() {
        let result = render_markdown("**bold text**");
        assert!(!result.is_empty());
        let found_bold = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::BOLD))
        });
        assert!(found_bold);
    }

    #[test]
    fn italic_text_has_italic_modifier() {
        let result = render_markdown("*italic text*");
        assert!(!result.is_empty());
        let found_italic = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::ITALIC))
        });
        assert!(found_italic);
    }

    #[test]
    fn strikethrough_text_has_crossed_out_modifier() {
        let result = render_markdown("~~strikethrough~~");
        assert!(!result.is_empty());
        let found_strikethrough = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::CROSSED_OUT))
        });
        assert!(found_strikethrough);
    }

    #[test]
    fn inline_code_has_green_color_and_backticks() {
        let result = render_markdown("Use `code` here");
        let found_code = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Green) && span.content.contains("`code`"))
        });
        assert!(found_code);
    }

    #[test]
    fn code_block_has_language_indicator() {
        let result = render_markdown("```rust\nlet x = 1;\n```");
        let has_lang_indicator = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("rust")));
        assert!(has_lang_indicator);
    }

    #[test]
    fn code_block_has_border_characters() {
        let result = render_markdown("```\ncode\n```");
        let has_top_border = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("┌")));
        let has_bottom_border = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("└")));
        assert!(has_top_border);
        assert!(has_bottom_border);
    }

    #[test]
    fn code_block_content_has_green_color() {
        let result = render_markdown("```\ncode line\n```");
        let has_green_code = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Green) && span.content.contains("code"))
        });
        assert!(has_green_code);
    }

    #[test]
    fn unordered_list_has_bullet_points() {
        let result = render_markdown("- item one\n- item two");
        let has_bullets = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("•")));
        assert!(has_bullets);
    }

    #[test]
    fn ordered_list_has_numbers() {
        let result = render_markdown("1. first\n2. second");
        let has_number_one = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("1.")));
        let has_number_two = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("2.")));
        assert!(has_number_one);
        assert!(has_number_two);
    }

    #[test]
    fn list_bullets_are_yellow() {
        let result = render_markdown("- item");
        let has_yellow_bullet = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Yellow) && span.content.contains("•"))
        });
        assert!(has_yellow_bullet);
    }

    #[test]
    fn blockquote_has_vertical_bar() {
        let result = render_markdown("> quoted text");
        let has_quote_bar = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("│")));
        assert!(has_quote_bar);
    }

    #[test]
    fn blockquote_text_is_italic_gray() {
        let result = render_markdown("> quoted text");
        let has_italic_gray = result.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.style.fg == Some(Color::Gray)
                    && span.style.add_modifier.contains(Modifier::ITALIC)
            })
        });
        assert!(has_italic_gray);
    }

    #[test]
    fn link_has_blue_underlined_style() {
        let result = render_markdown("[link text](http://example.com)");
        let has_link_style = result.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.style.fg == Some(Color::Blue)
                    && span.style.add_modifier.contains(Modifier::UNDERLINED)
            })
        });
        assert!(has_link_style);
    }

    #[test]
    fn horizontal_rule_renders() {
        let result = render_markdown("---");
        let has_rule = result
            .iter()
            .any(|line| line.spans.iter().any(|span| span.content.contains("────")));
        assert!(has_rule);
    }

    #[test]
    fn horizontal_rule_is_dark_gray() {
        let result = render_markdown("---");
        let has_dark_gray_rule = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.fg == Some(Color::DarkGray) && span.content.contains("────"))
        });
        assert!(has_dark_gray_rule);
    }

    #[test]
    fn multiple_paragraphs_separated_by_empty_lines() {
        let result = render_markdown("First paragraph.\n\nSecond paragraph.");
        // Should have content, empty line, content pattern
        assert!(result.len() >= 3);
    }

    #[test]
    fn nested_formatting_works() {
        let result = render_markdown("**bold and *italic* text**");
        let has_bold = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::BOLD))
        });
        assert!(has_bold);
    }

    #[test]
    fn trailing_empty_lines_removed() {
        let result = render_markdown("Text\n\n\n");
        // Should not end with empty spans
        if let Some(last) = result.last() {
            assert!(!last.spans.is_empty() || result.len() == 1);
        }
    }

    #[test]
    fn mention_is_bold_and_cyan() {
        let result = render_markdown("Hello @john.doe!");
        let has_mention_style = result.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.content.contains("@john.doe")
                    && span.style.fg == Some(Color::Cyan)
                    && span.style.add_modifier.contains(Modifier::BOLD)
            })
        });
        assert!(has_mention_style);
    }

    #[test]
    fn mention_with_underscores_and_hyphens() {
        let result = render_markdown("CC @jane_doe-smith");
        let has_mention = result.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.contains("@jane_doe-smith"))
        });
        assert!(has_mention);
    }

    #[test]
    fn multiple_mentions_highlighted() {
        let result = render_markdown("Hey @alice and @bob!");
        let mention_count: usize = result
            .iter()
            .flat_map(|line| line.spans.iter())
            .filter(|span| {
                span.content.starts_with('@')
                    && span.style.fg == Some(Color::Cyan)
                    && span.style.add_modifier.contains(Modifier::BOLD)
            })
            .count();
        assert_eq!(mention_count, 2);
    }

    #[test]
    fn lone_at_sign_not_styled_as_mention() {
        let result = render_markdown("Email me @ example.com");
        // The lone @ should not have cyan/bold styling
        let has_styled_lone_at = result.iter().any(|line| {
            line.spans.iter().any(|span| {
                span.content == "@"
                    && span.style.fg == Some(Color::Cyan)
                    && span.style.add_modifier.contains(Modifier::BOLD)
            })
        });
        assert!(!has_styled_lone_at);
    }

    #[test]
    fn text_around_mention_preserved() {
        let result = render_markdown("Before @user after");
        let spans: Vec<_> = result
            .iter()
            .flat_map(|line| line.spans.iter())
            .collect();
        // Should have "Before ", "@user", " after" as separate spans
        let has_before = spans.iter().any(|s| s.content.contains("Before"));
        let has_mention = spans.iter().any(|s| s.content.contains("@user"));
        let has_after = spans.iter().any(|s| s.content.contains("after"));
        assert!(has_before);
        assert!(has_mention);
        assert!(has_after);
    }
}
