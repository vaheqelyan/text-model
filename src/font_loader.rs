use num_traits::{clamp, clamp_max, clamp_min, sign};
use std::collections::HashMap;

fn draw_curve(
    curve: ft::outline::Curve,
    per_w: f32,
    per_h: f32,
    xmin: f32,
    ymin: f32,
    points: &mut Vec<(f32, f32)>,
    curve_points: &mut Vec<(f32, f32)>,
    full_points: &mut Vec<(f32, f32)>,
) {
    match curve {
        ft::outline::Curve::Line(pt) => {
            let diff_x = (xmin as f32 - pt.x as f32).abs();
            let diff_y = (ymin as f32 - -pt.y as f32).abs();

            points.push((diff_x * per_w, diff_y * per_h));

            full_points.push((diff_x * per_w, diff_y * per_h));
        }
        ft::outline::Curve::Bezier2(pt1, pt2) => {
            let diff_x_1 = (xmin as f32 - pt1.x as f32).abs();
            let diff_y_1 = (ymin as f32 - -pt1.y as f32).abs();

            let diff_x_2 = (xmin as f32 - pt2.x as f32).abs();
            let diff_y_2 = (ymin as f32 - -pt2.y as f32).abs();

            full_points.push((diff_x_1 * per_w, diff_y_1 * per_h));
        }
        _ => (),
    }
}

fn calculate_aspect_ratio_fit(
    srcWidth: f32,
    srcHeight: f32,
    maxWidth: f32,
    maxHeight: f32,
) -> RatioSize {
    let ratio = clamp_min(maxWidth / srcWidth, maxHeight / srcHeight);

    return RatioSize {
        width: srcWidth * ratio,
        height: srcHeight * ratio,
        ratio,
    };
}

#[derive(Debug)]
struct RatioSize {
    width: f32,
    height: f32,
    ratio: f32,
}

#[derive(Debug)]
pub struct FontSize {
    pub width: i64,
    pub height: i64,
    pub advance: i64,
    pub ascender: i16,
    pub descender: i16,
    pub points: Vec<(f32, f32)>,
}

pub fn create_font_map(font: &str) -> HashMap<char, FontSize> {
    // Freetype get measurements
    let library = ft::Library::init().unwrap();
    let face = library.new_face(font, 0).unwrap();
    face.set_char_size(40 * 64, 0, 50, 0).unwrap();

    let string = String::from("!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~");

    let mut font_measure: HashMap<char, FontSize> = HashMap::new();

    for c in string.chars() {
        face.load_char(c as usize, ft::face::LoadFlag::DEFAULT)
            .unwrap();
        let get_metrics = face.glyph().metrics();
        let get_advance = face.glyph().advance();

        let glyph = face.glyph();
        let metrics = glyph.metrics();
        let xmin = metrics.horiBearingX - 5;
        let width = metrics.width + 10;
        let ymin = -metrics.horiBearingY - 5;
        let height = metrics.height + 10;
        let outline = glyph.outline().unwrap();

        let z_width = metrics.width >> 6;
        let z_height = metrics.height >> 6;

        let fit = calculate_aspect_ratio_fit(width as f32, height as f32, 200.0, 200.0); //z_*

        let per_w = fit.width / width as f32;
        let per_h = fit.height / height as f32;

        let mut points: Vec<(f32, f32)> = vec![];
        let mut full_points: Vec<(f32, f32)> = vec![];
        let mut curve_point: Vec<(f32, f32)> = vec![];

        for contour in outline.contours_iter() {
            let start = contour.start();

            let diff_x = (xmin as f32 - start.x as f32).abs();
            let diff_y = (ymin as f32 - -start.y as f32).abs();

            points.push((diff_x * per_w, diff_y * per_h));

            full_points.push((diff_x * per_w, diff_y * per_h));

            for curve in contour {
                draw_curve(
                    curve,
                    per_w,
                    per_h,
                    xmin as f32,
                    ymin as f32,
                    &mut points,
                    &mut curve_point,
                    &mut full_points,
                );
            }
        }

        font_measure.insert(
            c,
            FontSize {
                width: get_metrics.width >> 6,
                height: get_metrics.height >> 6,
                advance: get_advance.x >> 6,
                // --
                ascender: face.ascender(),
                descender: face.descender(),
                points: full_points,
            },
        );
    }
    font_measure
}
