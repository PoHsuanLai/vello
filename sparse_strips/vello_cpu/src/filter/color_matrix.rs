// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Colour matrix filter implementation.

use super::FilterEffect;
use crate::filter::context::ScratchBuffer;
use vello_common::filter::color_matrix::ColorMatrix;
use vello_common::pixmap::Pixmap;

/// Both precision paths run the same arithmetic: the pixmap holds premultiplied 8-bit pixels
/// either way, and the matrix is evaluated in `f32` per pixel.
fn apply(matrix: &ColorMatrix, pixmap: &mut Pixmap) {
    for pixel in pixmap.data_mut() {
        *pixel = matrix.apply_premultiplied(*pixel);
    }
}

impl FilterEffect for ColorMatrix {
    fn execute_lowp(&self, pixmap: &mut Pixmap, _filter_scratch: &mut ScratchBuffer) {
        apply(self, pixmap);
    }

    fn execute_highp(&self, pixmap: &mut Pixmap, _filter_scratch: &mut ScratchBuffer) {
        apply(self, pixmap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vello_common::color::PremulRgba8;
    use vello_common::filter_effects::matrices;

    #[test]
    fn a_grayscale_matrix_desaturates_every_pixel_of_the_layer() {
        let mut pixmap = Pixmap::new(2, 2);
        pixmap.data_mut().fill(PremulRgba8 {
            r: 255,
            g: 0,
            b: 0,
            a: 255,
        });
        let mut scratch = ScratchBuffer::new();
        ColorMatrix::new(matrices::GRAYSCALE).execute_lowp(&mut pixmap, &mut scratch);
        for y in 0..2 {
            for x in 0..2 {
                assert_eq!(
                    pixmap.sample(x, y),
                    PremulRgba8 {
                        r: 54,
                        g: 54,
                        b: 54,
                        a: 255
                    }
                );
            }
        }
    }
}
