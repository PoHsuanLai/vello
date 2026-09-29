// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! The colour matrix filter.

use crate::color::PremulRgba8;

/// A 4x5 colour matrix applied to the non-premultiplied colour of every pixel (`feColorMatrix`
/// `type="matrix"`, and every CSS colour filter function: `brightness()`, `contrast()`,
/// `grayscale()`, `hue-rotate()`, `invert()`, `opacity()`, `saturate()`, `sepia()`).
///
/// Row `i` produces output channel `i` (R, G, B, A) from `[R, G, B, A, 1]`, all in `0.0..=1.0`;
/// results are clamped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorMatrix {
    /// The 20 coefficients, row-major.
    pub matrix: [f32; 20],
}

impl ColorMatrix {
    /// Create a colour matrix filter.
    pub fn new(matrix: [f32; 20]) -> Self {
        Self { matrix }
    }

    /// Transform a non-premultiplied colour, clamping every channel to `0.0..=1.0`.
    pub fn apply(&self, [r, g, b, a]: [f32; 4]) -> [f32; 4] {
        let m = &self.matrix;
        let row = |i: usize| {
            let o = i * 5;
            (m[o] * r + m[o + 1] * g + m[o + 2] * b + m[o + 3] * a + m[o + 4]).clamp(0.0, 1.0)
        };
        [row(0), row(1), row(2), row(3)]
    }

    /// Transform a premultiplied pixel: un-premultiply, [`apply`](Self::apply), premultiply.
    pub fn apply_premultiplied(&self, pixel: PremulRgba8) -> PremulRgba8 {
        let a = f32::from(pixel.a) / 255.0;
        // A transparent pixel has no colour to transform; matrices that raise alpha from zero
        // (an offset in the alpha row) still see black.
        let unpremultiply = |c: u8| {
            if pixel.a == 0 {
                0.0
            } else {
                (f32::from(c) / 255.0 / a).min(1.0)
            }
        };
        let [r, g, b, a] = self.apply([
            unpremultiply(pixel.r),
            unpremultiply(pixel.g),
            unpremultiply(pixel.b),
            a,
        ]);
        let to_u8 = |c: f32| (c * 255.0 + 0.5) as u8;
        PremulRgba8 {
            r: to_u8(r * a),
            g: to_u8(g * a),
            b: to_u8(b * a),
            a: to_u8(a),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filter_effects::matrices;

    const CASES: &[(&str, [f32; 20], PremulRgba8, PremulRgba8)] = &[
        (
            "the identity leaves an opaque pixel alone",
            matrices::IDENTITY,
            PremulRgba8 {
                r: 200,
                g: 100,
                b: 50,
                a: 255,
            },
            PremulRgba8 {
                r: 200,
                g: 100,
                b: 50,
                a: 255,
            },
        ),
        (
            "the identity leaves a half-transparent pixel alone",
            matrices::IDENTITY,
            PremulRgba8 {
                r: 100,
                g: 50,
                b: 25,
                a: 128,
            },
            PremulRgba8 {
                r: 100,
                g: 50,
                b: 25,
                a: 128,
            },
        ),
        (
            "grayscale turns red into its luminosity",
            matrices::GRAYSCALE,
            PremulRgba8 {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
            PremulRgba8 {
                r: 54,
                g: 54,
                b: 54,
                a: 255,
            },
        ),
        (
            "a transparent pixel stays transparent",
            matrices::GRAYSCALE,
            PremulRgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            PremulRgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        ),
        (
            "an offset in the colour rows brightens and is clamped",
            [
                1.0, 0.0, 0.0, 0.0, 0.5, //
                0.0, 1.0, 0.0, 0.0, 0.5, //
                0.0, 0.0, 1.0, 0.0, 0.5, //
                0.0, 0.0, 0.0, 1.0, 0.0,
            ],
            PremulRgba8 {
                r: 200,
                g: 100,
                b: 0,
                a: 255,
            },
            PremulRgba8 {
                r: 255,
                g: 228,
                b: 128,
                a: 255,
            },
        ),
    ];

    #[test]
    fn each_case_transforms_the_pixel() {
        for (name, matrix, input, expected) in CASES {
            let got = ColorMatrix::new(*matrix).apply_premultiplied(*input);
            let close = [
                got.r.abs_diff(expected.r),
                got.g.abs_diff(expected.g),
                got.b.abs_diff(expected.b),
                got.a.abs_diff(expected.a),
            ]
            .iter()
            .all(|d| *d <= 1);
            assert!(close, "{name}: got {got:?}, expected {expected:?}");
        }
    }

    /// The CSS filter functions, checked against the definitions in the Filter Effects spec
    /// (<https://drafts.fxtf.org/filter-effects/#filter-functions>) written out per channel,
    /// not as matrices.
    mod css_functions {
        use super::*;

        const COLORS: [[f32; 4]; 4] = [
            [0.8, 0.2, 0.1, 1.0],
            [0.1, 0.5, 0.9, 0.6],
            [0.0, 0.0, 0.0, 1.0],
            [1.0, 1.0, 1.0, 0.25],
        ];

        fn luma([r, g, b, _]: [f32; 4]) -> f32 {
            0.2126 * r + 0.7152 * g + 0.0722 * b
        }

        fn assert_close(got: [f32; 4], expected: [f32; 4], what: &str) {
            for (g, e) in got.iter().zip(expected) {
                assert!(
                    (g - e).abs() < 1e-4,
                    "{what}: got {got:?}, expected {expected:?}"
                );
            }
        }

        fn apply(matrix: [f32; 20], color: [f32; 4]) -> [f32; 4] {
            ColorMatrix::new(matrix).apply(color)
        }

        #[test]
        fn brightness_scales_the_colour_channels() {
            for amount in [0.0, 0.5, 1.0, 1.5] {
                for c @ [r, g, b, a] in COLORS {
                    let expected =
                        [r * amount, g * amount, b * amount, a].map(|v| v.clamp(0.0, 1.0));
                    assert_close(
                        apply(matrices::brightness(amount), c),
                        expected,
                        "brightness",
                    );
                }
            }
        }

        #[test]
        fn contrast_pivots_around_one_half() {
            for amount in [0.0, 0.5, 1.0, 2.0] {
                for c @ [r, g, b, a] in COLORS {
                    let f = |v: f32| (amount * v + (0.5 - 0.5 * amount)).clamp(0.0, 1.0);
                    assert_close(
                        apply(matrices::contrast(amount), c),
                        [f(r), f(g), f(b), a],
                        "contrast",
                    );
                }
            }
        }

        #[test]
        fn grayscale_mixes_towards_the_luminosity() {
            for amount in [0.0, 0.3, 1.0, 2.0] {
                let t = f32::min(amount, 1.0);
                for c @ [r, g, b, a] in COLORS {
                    let l = luma(c);
                    let mix = |v: f32| (1.0 - t) * v + t * l;
                    assert_close(
                        apply(matrices::grayscale(amount), c),
                        [mix(r), mix(g), mix(b), a],
                        "grayscale",
                    );
                }
            }
        }

        #[test]
        fn sepia_at_full_strength_is_the_sepia_matrix() {
            assert_eq!(
                matrices::sepia(1.0).map(|v| (v * 1e4).round()),
                matrices::SEPIA.map(|v| (v * 1e4).round())
            );
            assert_eq!(
                matrices::sepia(0.0).map(|v| (v * 1e4).round()),
                matrices::IDENTITY.map(|v| (v * 1e4).round())
            );
        }

        #[test]
        fn invert_mixes_towards_the_complement() {
            for amount in [0.0, 0.25, 1.0] {
                for c @ [r, g, b, a] in COLORS {
                    let f = |v: f32| amount * (1.0 - v) + (1.0 - amount) * v;
                    assert_close(
                        apply(matrices::invert(amount), c),
                        [f(r), f(g), f(b), a],
                        "invert",
                    );
                }
            }
        }

        #[test]
        fn opacity_scales_only_alpha() {
            for amount in [0.0, 0.4, 1.0] {
                for c @ [r, g, b, a] in COLORS {
                    assert_close(
                        apply(matrices::opacity(amount), c),
                        [r, g, b, a * amount],
                        "opacity",
                    );
                }
            }
        }

        #[test]
        fn saturate_interpolates_from_the_luminance_matrix() {
            // The spec's saturate coefficients (Rec. 601-ish 0.213/0.715/0.072).
            let l = |[r, g, b, _]: [f32; 4]| 0.213 * r + 0.715 * g + 0.072 * b;
            for amount in [0.0, 0.5, 1.0, 1.7] {
                for c @ [r, g, b, a] in COLORS {
                    let mix = |v: f32| (l(c) + amount * (v - l(c))).clamp(0.0, 1.0);
                    assert_close(
                        apply(matrices::saturate(amount), c),
                        [mix(r), mix(g), mix(b), a],
                        "saturate",
                    );
                }
            }
        }

        #[test]
        fn hue_rotate_is_identity_at_zero_and_full_turn_and_keeps_luminance() {
            for c in COLORS {
                assert_close(apply(matrices::hue_rotate(0.0), c), c, "hue-rotate(0)");
                assert_close(apply(matrices::hue_rotate(360.0), c), c, "hue-rotate(360)");
            }
            // The rotation keeps 0.213 R + 0.715 G + 0.072 B (before clamping): pick a colour
            // where nothing clamps.
            let c = [0.5, 0.5, 0.5, 1.0];
            for angle in [30.0, 90.0, 180.0, 270.0] {
                assert_close(
                    apply(matrices::hue_rotate(angle), c),
                    c,
                    "hue-rotate of grey",
                );
            }
            let c = [0.6, 0.4, 0.5, 1.0];
            let l = |[r, g, b, _]: [f32; 4]| 0.213 * r + 0.715 * g + 0.072 * b;
            let out = apply(matrices::hue_rotate(20.0), c);
            assert!((l(out) - l(c)).abs() < 1e-3, "luminance changed: {out:?}");
        }

        #[test]
        fn hue_rotate_by_a_half_turn_matches_the_closed_form() {
            // cos = -1, sin = 0.
            let [r, g, b, _] = [0.8, 0.2, 0.1, 1.0];
            let expected = [
                (0.213 - 0.787) * r + (0.715 + 0.715) * g + (0.072 + 0.072) * b,
                (0.213 + 0.213) * r + (0.715 - 0.285) * g + (0.072 + 0.072) * b,
                (0.213 + 0.213) * r + (0.715 + 0.715) * g + (0.072 - 0.928) * b,
                1.0,
            ]
            .map(|v: f32| v.clamp(0.0, 1.0));
            assert_close(
                apply(matrices::hue_rotate(180.0), [r, g, b, 1.0]),
                expected,
                "hue-rotate(180)",
            );
        }
    }
}
