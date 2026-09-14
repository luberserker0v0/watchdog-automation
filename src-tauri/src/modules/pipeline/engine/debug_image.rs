use chrono::Local;
use image::{DynamicImage, Rgb, RgbImage};
use imageproc::drawing::draw_filled_circle_mut;
use imageproc::drawing::draw_hollow_circle_mut;
use imageproc::drawing::draw_hollow_rect_mut;
use imageproc::drawing::draw_line_segment_mut;
use imageproc::rect::Rect;
use std::collections::HashMap;

use crate::modules::pipeline::models::action::Action;
use crate::modules::pipeline::models::recognizer::{MatchCandidate, MatchResult, Recognition};
use crate::utils::paths::debug_visions_dir;

const RANK_COLORS: &[(u8, u8, u8)] = &[
    (0, 255, 0),   // rank 0: green
    (255, 255, 0), // rank 1: yellow
    (255, 165, 0), // rank 2: orange
    (255, 0, 0),   // rank 3: red
    (255, 0, 255), // rank 4: magenta
    (0, 255, 255), // rank 5: cyan
    (0, 100, 255), // rank 6: blue
];

const COLOR_CYAN: Rgb<u8> = Rgb([0, 200, 255]);
const COLOR_ORANGE: Rgb<u8> = Rgb([255, 165, 0]);
const COLOR_YELLOW: Rgb<u8> = Rgb([255, 255, 0]);
const COLOR_MAGENTA: Rgb<u8> = Rgb([255, 0, 255]);
const COLOR_WHITE: Rgb<u8> = Rgb([255, 255, 255]);
const COLOR_RED: Rgb<u8> = Rgb([255, 0, 0]);
const COLOR_PURPLE: Rgb<u8> = Rgb([180, 0, 255]);
const COLOR_FOCUS_ROI: Rgb<u8> = Rgb([255, 184, 0]);
const COLOR_BOX: Rgb<u8> = Rgb([0, 255, 0]);
const COLOR_TARGET: Rgb<u8> = Rgb([0, 200, 255]);
const COLOR_DARK_BG: Rgb<u8> = Rgb([30, 30, 30]);
const MAX_ADDITIONAL_CANDIDATE_OVERLAYS: usize = 32;

fn rank_color(rank: usize) -> Rgb<u8> {
    let (r, g, b) = RANK_COLORS.get(rank).copied().unwrap_or(RANK_COLORS[6]);
    Rgb([r, g, b])
}

fn format_recognition_label(recognition: &Recognition) -> String {
    match recognition {
        Recognition::DirectHit => "DirectHit".to_string(),
        Recognition::TemplateMatch { template, .. } => format!("Template: {}", template),
        Recognition::Ocr { expected, .. } => match expected {
            Some(e) => format!("OCR \"{}\"", e),
            None => "OCR".to_string(),
        },
        Recognition::ColorMatch { .. } => "ColorMatch".to_string(),
        Recognition::Custom { recognition, .. } => format!("Custom: {}", recognition),
    }
}

fn format_action_label(action: &Action) -> String {
    match action {
        Action::DoNothing => "DoNothing".to_string(),
        Action::Click { .. } => "Click".to_string(),
        Action::LongPress { .. } => "LongPress".to_string(),
        Action::Swipe { .. } => "Swipe".to_string(),
        Action::Scroll { .. } => "Scroll".to_string(),
        Action::ClickKey { .. } => "ClickKey".to_string(),
        Action::InputText { .. } => "InputText".to_string(),
        Action::StopTask => "StopTask".to_string(),
        Action::Custom { action, .. } => format!("Custom: {}", action),
    }
}

fn format_action_labels(actions: &[Action]) -> String {
    if actions.is_empty() {
        return "DoNothing".to_string();
    }

    actions
        .iter()
        .map(format_action_label)
        .collect::<Vec<_>>()
        .join(" -> ")
}

pub struct DebugImageContext {
    pub attempt: usize,
    pub frame_sequence: u64,
    pub capture_target: String,
    pub capture_area: String,
    pub actions_run_afterward: bool,
}

fn debug_image_filename(
    node_id: &str,
    step: usize,
    timestamp: &str,
    context: Option<&DebugImageContext>,
) -> String {
    match context {
        Some(context) => format!(
            "{}_s{}_a{}_f{}_{}.jpg",
            node_id, step, context.attempt, context.frame_sequence, timestamp
        ),
        None => format!("{}_{}.jpg", node_id, timestamp),
    }
}

pub fn save_debug_image(
    screenshot: &DynamicImage,
    node_id: &str,
    step: usize,
    recognition: &Recognition,
    roi: Option<&[i32; 4]>,
    match_result: &MatchResult,
    actions: &[Action],
    recognized_regions: &HashMap<String, crate::modules::pipeline::models::recognizer::Region>,
    context: Option<&DebugImageContext>,
) -> Result<(), String> {
    let src = screenshot.to_rgb8();
    let original_w = src.width();
    let panel_w = 200u32;

    // Extend canvas horizontally for info panel sidebar
    let mut img = RgbImage::from_pixel(original_w + panel_w, src.height(), COLOR_DARK_BG);
    image::imageops::overlay(&mut img, &src, 0, 0);

    // Draw the recognition focus first. The default 0,0,0,0 sentinel is the
    // complete captured frame, so it remains visible even when recognition did
    // not produce a candidate.
    let focus_roi = crate::modules::pipeline::plugins::shared::resolve_recognition_roi(
        roi,
        original_w,
        src.height(),
    )
    .unwrap_or((0, 0, original_w, src.height()));
    draw_focus_roi(&mut img, focus_roi);

    // Keep the screenshot readable even when a permissive matcher returns many
    // objects. Draw representative unselected candidates first and the selected
    // candidate last so it can never be hidden by another outline.
    let mut drawn_candidate_count = 0usize;
    for candidate in match_result
        .candidates
        .iter()
        .filter(|candidate| !candidate.selected)
        .take(MAX_ADDITIONAL_CANDIDATE_OVERLAYS)
    {
        draw_roi_border(&mut img, &candidate, rank_color(candidate.rank));
        drawn_candidate_count += 1;
    }
    if let Some(selected) = match_result
        .candidates
        .iter()
        .find(|candidate| candidate.selected)
    {
        draw_roi_border(&mut img, selected, COLOR_BOX);
        drawn_candidate_count += 1;
    }

    // Box is the selected recognition result; Target is the action geometry
    // after resolving target and applying target_offset.
    if let Some(box_region) = &match_result.box_region {
        draw_region_border(&mut img, box_region, COLOR_BOX, 2);
    }

    // Draw every action in execution order. Nodes commonly click a resolved
    // target and then perform a non-positional action such as InputText; using
    // only the final action would hide the click position from the debug image.
    let action_targets = draw_action_symbols(
        &mut img,
        actions,
        match_result.box_region.as_ref(),
        recognized_regions,
    );

    // Draw info panel (right sidebar)
    draw_info_panel(
        &mut img,
        original_w as i32,
        recognition,
        focus_roi,
        match_result,
        drawn_candidate_count,
        &action_targets,
        actions,
        step,
        node_id,
        context,
    );

    // Save as JPEG
    let dir = debug_visions_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Failed to create debug dir: {}", e))?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S_%3f");
    let filename = debug_image_filename(node_id, step, &timestamp.to_string(), context);
    let path = dir.join(&filename);

    let jpeg_bytes = turbojpeg::compress_image(&img, 80, turbojpeg::Subsamp::None)
        .map_err(|e| format!("JPEG encode failed: {}", e))?;

    std::fs::write(&path, &jpeg_bytes)
        .map_err(|e| format!("Failed to write debug image: {}", e))?;

    log::info!("[debug_image] saved: {}", path.display());
    Ok(())
}

fn draw_focus_roi(img: &mut RgbImage, (x, y, width, height): (u32, u32, u32, u32)) {
    if width == 0 || height == 0 {
        return;
    }
    // Keep all three lines inside the captured image, including a full-frame ROI.
    for inset in 0..3u32 {
        if width <= inset * 2 || height <= inset * 2 {
            break;
        }
        let rect = Rect::at((x + inset) as i32, (y + inset) as i32)
            .of_size(width - inset * 2, height - inset * 2);
        draw_hollow_rect_mut(img, rect, COLOR_FOCUS_ROI);
    }
}

fn draw_roi_border(img: &mut RgbImage, candidate: &MatchCandidate, color: Rgb<u8>) {
    draw_region_border(img, &candidate.region, color, 1);
}

fn draw_region_border(
    img: &mut RgbImage,
    region: &crate::modules::pipeline::models::recognizer::Region,
    color: Rgb<u8>,
    thickness: u32,
) {
    if region.width == 0 || region.height == 0 {
        return;
    }
    for inset in 0..thickness {
        if region.width <= inset * 2 || region.height <= inset * 2 {
            break;
        }
        draw_hollow_rect_mut(
            img,
            Rect::at(region.x + inset as i32, region.y + inset as i32)
                .of_size(region.width - inset * 2, region.height - inset * 2),
            color,
        );
    }
}

fn draw_action_symbol(
    img: &mut RgbImage,
    action: &Action,
    box_region: Option<&crate::modules::pipeline::models::recognizer::Region>,
    recognized_regions: &HashMap<String, crate::modules::pipeline::models::recognizer::Region>,
) -> Vec<crate::modules::pipeline::models::recognizer::Region> {
    use crate::modules::pipeline::plugins::shared::{
        action_anchor, resolve_target_region_with_offset, validate_local_action_region,
    };
    let mut targets = Vec::new();
    let mut resolve = |target, offset| {
        resolve_target_region_with_offset(target, offset, box_region, recognized_regions)
            .ok()
            .inspect(|region| {
                let color = if action_anchor(region)
                    .and_then(|anchor| {
                        validate_local_action_region(&anchor, img.width(), img.height())
                    })
                    .is_ok()
                {
                    COLOR_TARGET
                } else {
                    COLOR_RED
                };
                draw_region_border(img, region, color, 2);
            })
    };
    match action {
        Action::Click {
            target,
            target_offset,
        } => {
            if let Some(region) = resolve(target, target_offset.as_ref()) {
                let (x, y) = region.center();
                draw_click(img, x, y);
                targets.push(region);
            }
        }
        Action::LongPress {
            target,
            target_offset,
            ..
        } => {
            if let Some(region) = resolve(target, target_offset.as_ref()) {
                let (x, y) = region.center();
                draw_long_press(img, x, y);
                targets.push(region);
            }
        }
        Action::Swipe {
            begin,
            begin_offset,
            end,
            end_offset,
            ..
        } => {
            let start = resolve(begin, begin_offset.as_ref());
            let end = resolve(end, end_offset.as_ref());
            if let (Some(start), Some(end)) = (start, end) {
                draw_swipe(img, start.center(), end.center());
                targets.push(start);
                targets.push(end);
            }
        }
        Action::Scroll {
            target,
            target_offset,
            dx,
            dy,
        } => {
            if let Some(region) = resolve(target, target_offset.as_ref()) {
                let (x, y) = region.center();
                draw_scroll(img, x, y, *dx, *dy);
                targets.push(region);
            }
        }
        Action::ClickKey { .. } => draw_key_icon(img),
        Action::InputText { .. } => draw_text_icon(img),
        Action::StopTask => draw_stop_icon(img),
        Action::Custom { .. } => draw_custom_icon(img),
        Action::DoNothing => {}
    }
    targets
}

fn draw_action_symbols(
    img: &mut RgbImage,
    actions: &[Action],
    box_region: Option<&crate::modules::pipeline::models::recognizer::Region>,
    recognized_regions: &HashMap<String, crate::modules::pipeline::models::recognizer::Region>,
) -> Vec<crate::modules::pipeline::models::recognizer::Region> {
    let mut targets = Vec::new();
    for action in actions {
        targets.extend(draw_action_symbol(
            img,
            action,
            box_region,
            recognized_regions,
        ));
    }
    targets
}

fn draw_click(img: &mut RgbImage, x: i32, y: i32) {
    draw_hollow_circle_mut(img, (x, y), 12, COLOR_CYAN);
    // Center dot
    let dot_rect = Rect::at(x - 1, y - 1).of_size(3, 3);
    imageproc::drawing::draw_filled_rect_mut(img, dot_rect, COLOR_CYAN);
}

fn draw_long_press(img: &mut RgbImage, x: i32, y: i32) {
    draw_filled_circle_mut(img, (x, y), 12, COLOR_ORANGE);
    // Inner ring
    draw_hollow_circle_mut(img, (x, y), 8, COLOR_WHITE);
}

fn draw_swipe(img: &mut RgbImage, start: (i32, i32), end: (i32, i32)) {
    // Start circle
    draw_hollow_circle_mut(img, start, 10, COLOR_YELLOW);

    // Arrow line
    draw_line_segment_mut(
        img,
        (start.0 as f32, start.1 as f32),
        (end.0 as f32, end.1 as f32),
        COLOR_YELLOW,
    );

    // Arrowhead
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let len = ((dx * dx + dy * dy) as f32).sqrt();
    if len > 0.0 {
        let ux = dx as f32 / len;
        let uy = dy as f32 / len;
        let arrow_len = 12.0_f32;
        let arrow_width = 6.0_f32;

        let tip = (end.0 as f32, end.1 as f32);
        let left = (
            tip.0 - ux * arrow_len + uy * arrow_width,
            tip.1 - uy * arrow_len - ux * arrow_width,
        );
        let right = (
            tip.0 - ux * arrow_len - uy * arrow_width,
            tip.1 - uy * arrow_len + ux * arrow_width,
        );

        draw_line_segment_mut(img, tip, left, COLOR_YELLOW);
        draw_line_segment_mut(img, tip, right, COLOR_YELLOW);
    }
}

fn draw_scroll(img: &mut RgbImage, x: i32, y: i32, dx: i32, dy: i32) {
    // Center circle
    draw_hollow_circle_mut(img, (x, y), 10, COLOR_MAGENTA);

    // Vertical arrow for dy
    if dy != 0 {
        // Enigo defines positive vertical scroll as down and negative as up.
        let arrow_end_y = if dy > 0 { y + 30 } else { y - 30 };
        draw_line_segment_mut(
            img,
            (x as f32, y as f32),
            (x as f32, arrow_end_y as f32),
            COLOR_MAGENTA,
        );
        // Arrowhead
        let head_dir = if dy > 0 { 1.0_f32 } else { -1.0_f32 };
        draw_line_segment_mut(
            img,
            (x as f32, arrow_end_y as f32),
            ((x - 6) as f32, (arrow_end_y as f32 - head_dir * 10.0)),
            COLOR_MAGENTA,
        );
        draw_line_segment_mut(
            img,
            (x as f32, arrow_end_y as f32),
            ((x + 6) as f32, (arrow_end_y as f32 - head_dir * 10.0)),
            COLOR_MAGENTA,
        );
    }

    // Horizontal arrow for dx
    if dx != 0 {
        let arrow_end_x = if dx > 0 { x + 30 } else { x - 30 };
        draw_line_segment_mut(
            img,
            (x as f32, y as f32),
            (arrow_end_x as f32, y as f32),
            COLOR_MAGENTA,
        );
        let head_dir = if dx > 0 { 1.0_f32 } else { -1.0_f32 };
        draw_line_segment_mut(
            img,
            (arrow_end_x as f32, y as f32),
            ((arrow_end_x as f32 - head_dir * 10.0), (y - 6) as f32),
            COLOR_MAGENTA,
        );
        draw_line_segment_mut(
            img,
            (arrow_end_x as f32, y as f32),
            ((arrow_end_x as f32 - head_dir * 10.0), (y + 6) as f32),
            COLOR_MAGENTA,
        );
    }
}

fn draw_key_icon(img: &mut RgbImage) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let cx = w / 2;
    let cy = h / 2;

    // Keyboard shape
    let rect = Rect::at(cx - 20, cy - 12).of_size(40, 24);
    imageproc::drawing::draw_hollow_rect_mut(img, rect, COLOR_WHITE);

    // "K" letter (simplified as lines)
    let kx = cx;
    let ky = cy;
    draw_line_segment_mut(
        img,
        (kx as f32 - 6.0, (ky - 8) as f32),
        (kx as f32 - 6.0, (ky + 8) as f32),
        COLOR_WHITE,
    );
    draw_line_segment_mut(
        img,
        (kx as f32 - 6.0, ky as f32),
        (kx as f32 + 6.0, (ky - 8) as f32),
        COLOR_WHITE,
    );
    draw_line_segment_mut(
        img,
        (kx as f32 - 6.0, ky as f32),
        (kx as f32 + 6.0, (ky + 8) as f32),
        COLOR_WHITE,
    );
}

fn draw_text_icon(img: &mut RgbImage) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let cx = w / 2;
    let cy = h / 2;

    // Text box
    let rect = Rect::at(cx - 16, cy - 16).of_size(32, 32);
    imageproc::drawing::draw_hollow_rect_mut(img, rect, COLOR_WHITE);

    // "T" letter
    draw_line_segment_mut(
        img,
        ((cx - 8) as f32, (cy - 10) as f32),
        ((cx + 8) as f32, (cy - 10) as f32),
        COLOR_WHITE,
    );
    draw_line_segment_mut(
        img,
        (cx as f32, (cy - 10) as f32),
        (cx as f32, (cy + 10) as f32),
        COLOR_WHITE,
    );
}

fn draw_stop_icon(img: &mut RgbImage) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let cx = w / 2;
    let cy = h / 2;

    // Stop box
    let rect = Rect::at(cx - 16, cy - 16).of_size(32, 32);
    imageproc::drawing::draw_hollow_rect_mut(img, rect, COLOR_RED);

    // X mark
    draw_line_segment_mut(
        img,
        ((cx - 8) as f32, (cy - 8) as f32),
        ((cx + 8) as f32, (cy + 8) as f32),
        COLOR_RED,
    );
    draw_line_segment_mut(
        img,
        ((cx + 8) as f32, (cy - 8) as f32),
        ((cx - 8) as f32, (cy + 8) as f32),
        COLOR_RED,
    );
}

fn draw_custom_icon(img: &mut RgbImage) {
    let w = img.width() as i32;
    let h = img.height() as i32;
    let cx = w / 2;
    let cy = h / 2;

    // Custom box
    let rect = Rect::at(cx - 16, cy - 16).of_size(32, 32);
    imageproc::drawing::draw_hollow_rect_mut(img, rect, COLOR_PURPLE);

    // "?" mark (simplified as dot + arc lines)
    draw_line_segment_mut(
        img,
        (cx as f32, (cy - 10) as f32),
        (cx as f32, (cy - 2) as f32),
        COLOR_PURPLE,
    );
    draw_line_segment_mut(
        img,
        (cx as f32, (cy + 4) as f32),
        (cx as f32, (cy + 6) as f32),
        COLOR_PURPLE,
    );
    draw_filled_circle_mut(img, (cx, cy + 8), 2, COLOR_PURPLE);
}

fn draw_info_panel(
    img: &mut RgbImage,
    offset_x: i32,
    recognition: &Recognition,
    focus_roi: (u32, u32, u32, u32),
    match_result: &MatchResult,
    drawn_candidate_count: usize,
    action_targets: &[crate::modules::pipeline::models::recognizer::Region],
    actions: &[Action],
    step: usize,
    node_id: &str,
    context: Option<&DebugImageContext>,
) {
    let panel_w = 200;
    let panel_h = img.height() as i32;
    let px = offset_x;
    let py = 0;

    // Background
    for y in py..py + panel_h {
        for x in px..px + panel_w {
            if x >= 0 && y >= 0 && (x as u32) < img.width() && (y as u32) < img.height() {
                img.put_pixel(x as u32, y as u32, COLOR_DARK_BG);
            }
        }
    }

    // Border (left edge of panel)
    let border_rect = Rect::at(px, py).of_size(panel_w as u32, panel_h as u32);
    draw_hollow_rect_mut(img, border_rect, COLOR_WHITE);

    // Recognition label (line 1)
    let recog_label = format_recognition_label(recognition);
    draw_text_line(img, px + 8, py + 20, &recog_label, COLOR_WHITE);

    // Recognition focus (line 2)
    let roi_label = format!(
        "ROI {},{} {}x{}",
        focus_roi.0, focus_roi.1, focus_roi.2, focus_roi.3
    );
    draw_text_line(img, px + 8, py + 40, &roi_label, COLOR_FOCUS_ROI);

    // Selected recognition box (line 3)
    let box_label = match match_result.box_region.as_ref() {
        Some(region) => format!(
            "BOX {},{} {}x{}",
            region.x, region.y, region.width, region.height
        ),
        None => "BOX none".to_string(),
    };
    draw_text_line(img, px + 8, py + 60, &box_label, COLOR_BOX);

    // Final action target after target_offset (line 4)
    let target_label = match action_targets.first() {
        Some(region) => format!(
            "TARGET {},{} {}x{}",
            region.x, region.y, region.width, region.height
        ),
        None => "TARGET none".to_string(),
    };
    draw_text_line(img, px + 8, py + 80, &target_label, COLOR_TARGET);

    let raw_count = match_result
        .raw_candidate_count
        .unwrap_or(match_result.candidates.len());
    draw_text_line(
        img,
        px + 8,
        py + 100,
        &format!("RAW {}", raw_count),
        COLOR_WHITE,
    );
    draw_text_line(
        img,
        px + 8,
        py + 120,
        &format!(
            "DISTINCT {}",
            match_result
                .distinct_candidate_count
                .unwrap_or(match_result.candidates.len())
        ),
        COLOR_WHITE,
    );
    let overlay_label = if drawn_candidate_count < match_result.candidates.len() {
        format!(
            "DRAWN {}/{} LIMITED",
            drawn_candidate_count,
            match_result.candidates.len()
        )
    } else {
        format!(
            "DRAWN {}/{}",
            drawn_candidate_count,
            match_result.candidates.len()
        )
    };
    draw_text_line(img, px + 8, py + 140, &overlay_label, COLOR_WHITE);

    // Match status
    let selected_rank = match_result
        .candidates
        .iter()
        .find(|candidate| candidate.selected)
        .map(|candidate| format!(" #{}", candidate.rank))
        .unwrap_or_default();
    let match_label = if match_result.matched {
        format!("YES{} ({:.3})", selected_rank, match_result.confidence)
    } else {
        format!("NO ({:.3})", match_result.confidence)
    };
    let match_color = if match_result.matched {
        Rgb([0, 255, 0])
    } else {
        COLOR_RED
    };
    draw_text_line(img, px + 8, py + 160, &match_label, match_color);

    // Action (line 7)
    let action_label = format_action_labels(actions);
    draw_text_line(img, px + 8, py + 180, &action_label, COLOR_YELLOW);

    // Step number (line 8)
    let step_label = format!("Step #{}", step);
    draw_text_line(img, px + 8, py + 200, &step_label, COLOR_WHITE);

    draw_text_line(
        img,
        px + 8,
        py + 220,
        &format!("Node {}", node_id),
        COLOR_WHITE,
    );
    if let Some(context) = context {
        draw_text_line(
            img,
            px + 8,
            py + 240,
            &format!(
                "Attempt {} Frame {}",
                context.attempt, context.frame_sequence
            ),
            COLOR_WHITE,
        );
        draw_text_line(
            img,
            px + 8,
            py + 260,
            &format!("{} {}", context.capture_target, context.capture_area),
            COLOR_WHITE,
        );
        draw_text_line(
            img,
            px + 8,
            py + 280,
            if context.actions_run_afterward {
                "Actions run afterward"
            } else {
                "No actions afterward"
            },
            COLOR_YELLOW,
        );
    }
}

/// Simple 5x7 bitmap font for ASCII characters (0-9, A-Z, basic symbols)
const FONT: &[(char, [u8; 7])] = &[
    (
        '0',
        [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
    ),
    (
        '1',
        [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
    ),
    (
        '2',
        [
            0b01110, 0b10001, 0b00001, 0b00110, 0b01000, 0b10000, 0b11111,
        ],
    ),
    (
        '3',
        [
            0b01110, 0b10001, 0b00001, 0b00110, 0b00001, 0b10001, 0b01110,
        ],
    ),
    (
        '4',
        [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
    ),
    (
        '5',
        [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
    ),
    (
        '6',
        [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        '7',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
    ),
    (
        '8',
        [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        '9',
        [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
    ),
    (
        'A',
        [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'B',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
    ),
    (
        'C',
        [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
    ),
    (
        'D',
        [
            0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100,
        ],
    ),
    (
        'E',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
    ),
    (
        'F',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
    ),
    (
        'G',
        [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
    ),
    (
        'H',
        [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'I',
        [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
    ),
    (
        'J',
        [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
    ),
    (
        'K',
        [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
    ),
    (
        'L',
        [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
    ),
    (
        'M',
        [
            0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'N',
        [
            0b10001, 0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001,
        ],
    ),
    (
        'O',
        [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        'P',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
    ),
    (
        'Q',
        [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b01110, 0b00001,
        ],
    ),
    (
        'R',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
    ),
    (
        'S',
        [
            0b01110, 0b10001, 0b10000, 0b01110, 0b00001, 0b10001, 0b01110,
        ],
    ),
    (
        'T',
        [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
    ),
    (
        'U',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        'V',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
    ),
    (
        'W',
        [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
    ),
    (
        'X',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
    ),
    (
        'Y',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
    ),
    (
        'Z',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
    ),
    (
        '.',
        [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100,
        ],
    ),
    (
        ':',
        [
            0b00000, 0b00100, 0b00000, 0b00000, 0b00000, 0b00100, 0b00000,
        ],
    ),
    (
        ' ',
        [
            0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
    ),
    (
        '(',
        [
            0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010,
        ],
    ),
    (
        ')',
        [
            0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000,
        ],
    ),
    (
        '#',
        [
            0b01010, 0b01010, 0b11111, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
    ),
    (
        '"',
        [
            0b01010, 0b01010, 0b01010, 0b00000, 0b00000, 0b00000, 0b00000,
        ],
    ),
    (
        '-',
        [
            0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000,
        ],
    ),
    (
        '+',
        [
            0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000,
        ],
    ),
    (
        '/',
        [
            0b00001, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b10000,
        ],
    ),
];

fn get_char_bitmap(ch: char) -> [u8; 7] {
    let upper = ch.to_ascii_uppercase();
    FONT.iter()
        .find(|(c, _)| *c == ch || *c == upper)
        .map(|(_, bits)| *bits)
        .unwrap_or([
            0b11111, 0b10001, 0b00100, 0b01000, 0b01000, 0b00000, 0b01000,
        ]) // "?" fallback
}

fn draw_text_line(img: &mut RgbImage, x: i32, y: i32, text: &str, color: Rgb<u8>) {
    let mut cursor_x = x;
    for ch in text.chars() {
        let bitmap = get_char_bitmap(ch);
        for row in 0..7 {
            for col in 0..5 {
                if bitmap[row] & (1 << (4 - col)) != 0 {
                    let px = cursor_x + col;
                    let py_pos = y - 7 + row as i32;
                    if px >= 0
                        && py_pos >= 0
                        && (px as u32) < img.width()
                        && (py_pos as u32) < img.height()
                    {
                        img.put_pixel(px as u32, py_pos as u32, color);
                    }
                }
            }
        }
        cursor_x += 6; // 5px char + 1px spacing
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::pipeline::models::action::Target;
    use crate::modules::pipeline::models::recognizer::Region;

    #[test]
    fn debug_action_target_uses_target_and_all_region_offset_values() {
        let mut image = RgbImage::new(400, 400);
        let box_region = Region {
            x: 100,
            y: 100,
            width: 80,
            height: 40,
        };
        let action = Action::Click {
            target: Target::RecognizedBool(true),
            target_offset: Some([10, 20, 20, 10]),
        };
        let targets = draw_action_symbol(&mut image, &action, Some(&box_region), &HashMap::new());

        assert_eq!(
            targets,
            vec![Region {
                x: 110,
                y: 120,
                width: 100,
                height: 50,
            }]
        );
        assert_eq!(targets[0].center(), (160, 145));
    }

    #[test]
    fn scroll_arrow_uses_enigo_vertical_direction() {
        let mut down = RgbImage::new(100, 100);
        draw_scroll(&mut down, 50, 50, 0, 1);
        assert_eq!(*down.get_pixel(50, 80), COLOR_MAGENTA);
        assert_eq!(*down.get_pixel(50, 20), Rgb([0, 0, 0]));

        let mut up = RgbImage::new(100, 100);
        draw_scroll(&mut up, 50, 50, 0, -1);
        assert_eq!(*up.get_pixel(50, 20), COLOR_MAGENTA);
        assert_eq!(*up.get_pixel(50, 80), Rgb([0, 0, 0]));
    }

    #[test]
    fn action_sequence_keeps_click_symbol_before_input_text() {
        let mut image = RgbImage::new(400, 400);
        let box_region = Region {
            x: 100,
            y: 100,
            width: 80,
            height: 40,
        };
        let actions = vec![
            Action::Click {
                target: Target::RecognizedBool(true),
                target_offset: Some([0, 38, 0, 0]),
            },
            Action::InputText {
                input_text: "account".to_string(),
            },
        ];

        let targets = draw_action_symbols(&mut image, &actions, Some(&box_region), &HashMap::new());

        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].center(), (140, 158));
        assert_eq!(*image.get_pixel(140, 158), COLOR_CYAN);
        assert_eq!(format_action_labels(&actions), "Click -> InputText");
    }

    #[test]
    fn debug_filename_distinguishes_every_node_attempt_and_frame() {
        let context = DebugImageContext {
            attempt: 2,
            frame_sequence: 17,
            capture_target: "Window 42".to_string(),
            capture_area: "client".to_string(),
            actions_run_afterward: false,
        };

        assert_eq!(
            debug_image_filename("InputAccount", 4, "20260914_010203_456", Some(&context)),
            "InputAccount_s4_a2_f17_20260914_010203_456.jpg"
        );
    }
}
