// Copyright 2026 the Vello Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Filter layers must render the same with the multi-threaded dispatcher as with the
//! single-threaded one.

#![cfg(feature = "multithreading")]

use vello_common::filter_effects::{Filter, FilterFunction, FilterPrimitive, matrices};
use vello_cpu::color::palette::css::{BLUE, GREEN, RED, YELLOW};
use vello_cpu::kurbo::{Circle, Rect, Shape};
use vello_cpu::{Level, Pixmap, RenderContext, RenderSettings, Resources};

const SIZE: u16 = 96;

fn render(threads: u16, scene: impl Fn(&mut RenderContext)) -> Pixmap {
    let mut ctx = RenderContext::new_with(
        SIZE,
        SIZE,
        RenderSettings {
            level: Level::baseline(),
            num_threads: threads,
        },
    );
    scene(&mut ctx);
    let mut pixmap = Pixmap::new(SIZE, SIZE);
    ctx.flush();
    ctx.render(pixmap.as_mut(), &mut Resources::new());
    pixmap
}

fn assert_same_on_both_dispatchers(scene: impl Fn(&mut RenderContext)) {
    let single = render(0, &scene);
    let multi = render(3, &scene);
    assert_eq!(
        single.data(),
        multi.data(),
        "the multi-threaded dispatcher renders differently"
    );
    // Guard against both being blank.
    assert!(
        single.data().iter().any(|p| p.a != 0),
        "the scene rendered nothing"
    );
}

fn rect(
    ctx: &mut RenderContext,
    x: f64,
    paint: vello_cpu::color::AlphaColor<vello_cpu::color::Srgb>,
) {
    ctx.set_paint(paint);
    ctx.fill_rect(&Rect::new(x, 20.0, x + 40.0, 70.0));
}

#[test]
fn blur_layer_between_plain_draws() {
    assert_same_on_both_dispatchers(|ctx| {
        rect(ctx, 0.0, RED);
        ctx.push_filter_layer(Filter::from_function(FilterFunction::Blur { radius: 3.0 }));
        rect(ctx, 20.0, GREEN);
        ctx.pop_layer();
        rect(ctx, 56.0, BLUE);
    });
}

#[test]
fn color_matrix_layer_with_nested_layer_and_outer_clip() {
    assert_same_on_both_dispatchers(|ctx| {
        ctx.push_clip_path(&Circle::new((48.0, 48.0), 44.0).to_path(0.1));
        rect(ctx, 0.0, YELLOW);
        ctx.push_filter_layer(Filter::from_primitive(FilterPrimitive::ColorMatrix {
            matrix: matrices::GRAYSCALE,
        }));
        rect(ctx, 10.0, RED);
        ctx.push_opacity_layer(0.5);
        rect(ctx, 30.0, BLUE);
        ctx.pop_layer();
        ctx.push_clip_layer(&Circle::new((60.0, 40.0), 20.0).to_path(0.1));
        rect(ctx, 40.0, GREEN);
        ctx.pop_layer();
        ctx.pop_layer();
        ctx.pop_clip_path();
        rect(ctx, 50.0, BLUE.with_alpha(0.5));
    });
}

#[test]
fn nested_filter_layers() {
    assert_same_on_both_dispatchers(|ctx| {
        ctx.push_filter_layer(Filter::from_function(FilterFunction::Blur { radius: 2.0 }));
        rect(ctx, 5.0, RED);
        ctx.push_filter_layer(Filter::from_primitive(FilterPrimitive::DropShadow {
            dx: 5.0,
            dy: 5.0,
            std_deviation: 2.0,
            color: BLUE,
            edge_mode: vello_common::filter_effects::EdgeMode::default(),
        }));
        rect(ctx, 30.0, GREEN);
        ctx.pop_layer();
        ctx.pop_layer();
    });
}

#[test]
fn a_filter_layer_can_be_rendered_repeatedly() {
    let mut ctx = RenderContext::new_with(
        SIZE,
        SIZE,
        RenderSettings {
            level: Level::baseline(),
            num_threads: 2,
        },
    );
    let mut first = Pixmap::new(SIZE, SIZE);
    let mut second = Pixmap::new(SIZE, SIZE);
    for out in [&mut first, &mut second] {
        ctx.reset();
        ctx.push_filter_layer(Filter::from_function(FilterFunction::Blur { radius: 2.0 }));
        rect(&mut ctx, 20.0, RED);
        ctx.pop_layer();
        ctx.flush();
        ctx.render(out.as_mut(), &mut Resources::new());
    }
    assert_eq!(first.data(), second.data());
}

#[test]
fn a_plain_scene_can_be_rendered_repeatedly() {
    let mut ctx = RenderContext::new_with(
        SIZE,
        SIZE,
        RenderSettings {
            level: Level::baseline(),
            num_threads: 2,
        },
    );
    for _ in 0..2 {
        ctx.reset();
        rect(&mut ctx, 20.0, RED);
        ctx.flush();
        ctx.render(Pixmap::new(SIZE, SIZE).as_mut(), &mut Resources::new());
    }
}
