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
            PremulRgba8 { r: 200, g: 100, b: 50, a: 255 },
            PremulRgba8 { r: 200, g: 100, b: 50, a: 255 },
        ),
        (
            "the identity leaves a half-transparent pixel alone",
            matrices::IDENTITY,
            PremulRgba8 { r: 100, g: 50, b: 25, a: 128 },
            PremulRgba8 { r: 100, g: 50, b: 25, a: 128 },
        ),
        (
            "grayscale turns red into its luminosity",
            matrices::GRAYSCALE,
            PremulRgba8 { r: 255, g: 0, b: 0, a: 255 },
            PremulRgba8 { r: 54, g: 54, b: 54, a: 255 },
        ),
        (
            "a transparent pixel stays transparent",
            matrices::GRAYSCALE,
            PremulRgba8 { r: 0, g: 0, b: 0, a: 0 },
            PremulRgba8 { r: 0, g: 0, b: 0, a: 0 },
        ),
        (
            "an offset in the colour rows brightens and is clamped",
            [
                1.0, 0.0, 0.0, 0.0, 0.5, //
                0.0, 1.0, 0.0, 0.0, 0.5, //
                0.0, 0.0, 1.0, 0.0, 0.5, //
                0.0, 0.0, 0.0, 1.0, 0.0,
            ],
            PremulRgba8 { r: 200, g: 100, b: 0, a: 255 },
            PremulRgba8 { r: 255, g: 228, b: 128, a: 255 },
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
}
