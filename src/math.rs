use bevy::prelude::*;

use crate::{NormalizedRect, PhysicalRect, SplitScreenPadding};

pub(crate) fn smoothstep(value: f32) -> f32 {
    let clamped = value.clamp(0.0, 1.0);
    clamped * clamped * (3.0 - 2.0 * clamped)
}

pub(crate) fn usable_target_size(target_size: UVec2, padding: SplitScreenPadding) -> UVec2 {
    UVec2::new(
        target_size
            .x
            .saturating_sub(padding.left.saturating_add(padding.right)),
        target_size
            .y
            .saturating_sub(padding.top.saturating_add(padding.bottom)),
    )
}

pub(crate) fn fraction_from_min_size(min_size: u32, total_size: u32) -> f32 {
    if total_size == 0 {
        0.0
    } else {
        (min_size as f32 / total_size as f32).clamp(0.0, 0.5)
    }
}

pub(crate) fn normalized_to_physical(
    rect: NormalizedRect,
    target_size: UVec2,
    padding: SplitScreenPadding,
) -> PhysicalRect {
    let usable = usable_target_size(target_size, padding);
    let padded_origin = UVec2::new(padding.left, padding.top);

    let min = Vec2::new(
        rect.min.x.clamp(0.0, 1.0) * usable.x as f32,
        rect.min.y.clamp(0.0, 1.0) * usable.y as f32,
    );
    let max = Vec2::new(
        rect.max.x.clamp(0.0, 1.0) * usable.x as f32,
        rect.max.y.clamp(0.0, 1.0) * usable.y as f32,
    );

    let position = padded_origin + min.round().as_uvec2();
    let max_u = padded_origin + max.round().as_uvec2();
    let mut size = max_u.saturating_sub(position);

    if rect.width() > 0.0 {
        size.x = size.x.max(1);
    }
    if rect.height() > 0.0 {
        size.y = size.y.max(1);
    }

    PhysicalRect { position, size }
}

pub(crate) fn normalized_point_to_physical(
    point: Vec2,
    target_size: UVec2,
    padding: SplitScreenPadding,
) -> Vec2 {
    let usable = usable_target_size(target_size, padding).as_vec2();
    let padded_origin = Vec2::new(padding.left as f32, padding.top as f32);

    padded_origin
        + Vec2::new(
            point.x.clamp(0.0, 1.0) * usable.x,
            point.y.clamp(0.0, 1.0) * usable.y,
        )
}
