# raw-lite Interface

## Direction

A compact archival negative sleeve rather than a photo editor: one destination, one large intake area, and a linear conversion status. The interface should feel understandable at a glance and stay out of the way.

## Palette

- Cool paper ground: `#dfe4e1`
- Light sleeve surface: `#f4f6f3`
- Ink green: `#17251f`
- Secondary ink: `#526159`
- Rules: `#8b9790`
- Active conversion / drop signal: `#d45b31`
- Keyboard focus: `#176b50`

## Type and layout

Use the operating system sans stack. The product name is the only display-scale text; all controls remain workmanlike. Thin rules, square geometry, and inset sleeve rails organize the screen. The drop field owns the center of the first viewport; advanced settings remain collapsed below it.

## Components and states

- Primary buttons use ink fill, light text, a 3px radius, and a soft downward shadow.
- Secondary file selection is an outlined button inside the drop field.
- Drop-ready state darkens the rule; drag-over changes the rule and surface to amber-tinted active color.
- Busy state disables every mutable control.
- Progress uses a native semantic `<progress>` element with tabular counts.
- Errors and folder-opening warnings use dark rust text, never color alone without copy.
- Keyboard focus uses a visible green outline. Reduced-motion users receive effectively instant transitions.

## Responsive behavior

Below 600px, stack destination controls, make the folder button full width, reduce outer gutters, and collapse advanced settings to one column. Preserve the drop field as the dominant task surface.
