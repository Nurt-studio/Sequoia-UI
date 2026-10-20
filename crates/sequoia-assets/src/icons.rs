pub(crate) fn builtin() -> &'static [(&'static str, &'static str)] {
    &[
        (
            "chevron-down",
            r#"<svg viewBox="0 0 24 24"><path d="M4 7 L12 15 L20 7 L20 10 L12 18 L4 10 Z"/></svg>"#,
        ),
        (
            "chevron-up",
            r#"<svg viewBox="0 0 24 24"><path d="M4 17 L12 9 L20 17 L20 14 L12 6 L4 14 Z"/></svg>"#,
        ),
        (
            "chevron-left",
            r#"<svg viewBox="0 0 24 24"><path d="M15 4 L7 12 L15 20 L12 20 L4 12 L12 4 Z"/></svg>"#,
        ),
        (
            "chevron-right",
            r#"<svg viewBox="0 0 24 24"><path d="M9 4 L17 12 L9 20 L12 20 L20 12 L12 4 Z"/></svg>"#,
        ),
        (
            "check",
            r#"<svg viewBox="0 0 24 24"><path d="M9.5 16.2 L5.3 12 L4 13.3 L9.5 18.8 L20 8.3 L18.7 7 Z"/></svg>"#,
        ),
        (
            "close",
            r#"<svg viewBox="0 0 24 24"><path d="M18.3 7.1 L16.9 5.7 L12 10.6 L7.1 5.7 L5.7 7.1 L10.6 12 L5.7 16.9 L7.1 18.3 L12 13.4 L16.9 18.3 L18.3 16.9 L13.4 12 Z"/></svg>"#,
        ),
        (
            "plus",
            r#"<svg viewBox="0 0 24 24"><path d="M11 5 H13 V11 H19 V13 H13 V19 H11 V13 H5 V11 H11 Z"/></svg>"#,
        ),
        (
            "minus",
            r#"<svg viewBox="0 0 24 24"><path d="M5 11 H19 V13 H5 Z"/></svg>"#,
        ),
        (
            "search",
            r#"<svg viewBox="0 0 24 24"><path d="M4 11 a7 7 0 1 0 14 0 a7 7 0 1 0 -14 0 Z M15.95 15.95 L20.45 20.45 L21.86 19.04 L17.36 14.54 Z"/></svg>"#,
        ),
        (
            "play",
            r#"<svg viewBox="0 0 24 24"><path d="M8 5 L19 12 L8 19 Z"/></svg>"#,
        ),
        (
            "pause",
            r#"<svg viewBox="0 0 24 24"><path d="M7 5 H10.5 V19 H7 Z M13.5 5 H17 V19 H13.5 Z"/></svg>"#,
        ),
        (
            "folder",
            r#"<svg viewBox="0 0 24 24"><path d="M3 6 C3 5.4 3.4 5 4 5 H9.5 L11.5 7 H20 C20.6 7 21 7.4 21 8 V18 C21 18.6 20.6 19 20 19 H4 C3.4 19 3 18.6 3 18 Z"/></svg>"#,
        ),
        (
            "file",
            r#"<svg viewBox="0 0 24 24"><path d="M6 3 H14 L18 7 V21 H6 Z"/></svg>"#,
        ),
        (
            "sliders",
            r#"<svg viewBox="0 0 24 24">
                 <path d="M4 5 H20 V7 H4 Z M4 11 H20 V13 H4 Z M4 17 H20 V19 H4 Z"/>
                 <path fill-rule="evenodd" d="M12 6 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0 Z M13.8 6 a1.2 1.2 0 1 0 2.4 0 a1.2 1.2 0 1 0 -2.4 0 Z
                                              M6 12 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0 Z M7.8 12 a1.2 1.2 0 1 0 2.4 0 a1.2 1.2 0 1 0 -2.4 0 Z
                                              M14 18 a3 3 0 1 0 6 0 a3 3 0 1 0 -6 0 Z M15.8 18 a1.2 1.2 0 1 0 2.4 0 a1.2 1.2 0 1 0 -2.4 0 Z"/>
               </svg>"#,
        ),
        (
            "trash",
            r#"<svg viewBox="0 0 24 24"><path d="M9 3 H15 V5 H20.5 V7 H3.5 V5 H9 Z"/><path d="M6 8 H18 L17.2 21 H6.8 Z"/></svg>"#,
        ),
        (
            "star",
            r#"<svg viewBox="0 0 24 24"><path d="M12 2 L14.9 8.6 L22 9.3 L16.7 14 L18.2 21 L12 17.3 L5.8 21 L7.3 14 L2 9.3 L9.1 8.6 Z"/></svg>"#,
        ),
        (
            "move",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20"/><path d="m15 19-3 3-3-3"/><path d="m19 9 3 3-3 3"/><path d="M2 12h20"/><path d="m5 9-3 3 3 3"/><path d="m9 5 3-3 3 3"/></svg>"#,
        ),
        (
            "mouse-pointer",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.037 4.688a.495.495 0 0 1 .651-.651l16 6.5a.5.5 0 0 1-.063.947l-6.124 1.58a2 2 0 0 0-1.438 1.435l-1.579 6.126a.5.5 0 0 1-.947.063z"/></svg>"#,
        ),
        (
            "frame",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="22" x2="2" y1="6" y2="6"/><line x1="22" x2="2" y1="18" y2="18"/><line x1="6" x2="6" y1="2" y2="22"/><line x1="18" x2="18" y1="2" y2="22"/></svg>"#,
        ),
        (
            "square",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2"/></svg>"#,
        ),
        (
            "circle",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/></svg>"#,
        ),
        (
            "type",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v16"/><path d="M4 7V5a1 1 0 0 1 1-1h14a1 1 0 0 1 1 1v2"/><path d="M9 20h6"/></svg>"#,
        ),
        (
            "hand",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2"/><path d="M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2"/><path d="M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8"/><path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15"/></svg>"#,
        ),
        (
            "eye",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/></svg>"#,
        ),
        (
            "eye-off",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/></svg>"#,
        ),
        (
            "align-center-horizontal",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 12h20"/><path d="M10 16v4a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2v-4"/><path d="M10 8V4a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v4"/><path d="M20 16v1a2 2 0 0 1-2 2h-2a2 2 0 0 1-2-2v-1"/><path d="M14 8V7c0-1.1.9-2 2-2h2a2 2 0 0 1 2 2v1"/></svg>"#,
        ),
        (
            "align-center-vertical",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2v20"/><path d="M8 10H4a2 2 0 0 1-2-2V6c0-1.1.9-2 2-2h4"/><path d="M16 10h4a2 2 0 0 0 2-2V6a2 2 0 0 0-2-2h-4"/><path d="M8 20H7a2 2 0 0 1-2-2v-2c0-1.1.9-2 2-2h1"/><path d="M16 14h1a2 2 0 0 1 2 2v2a2 2 0 0 1-2 2h-1"/></svg>"#,
        ),
        (
            "maximize",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 3H5a2 2 0 0 0-2 2v3"/><path d="M21 8V5a2 2 0 0 0-2-2h-3"/><path d="M3 16v3a2 2 0 0 0 2 2h3"/><path d="M16 21h3a2 2 0 0 0 2-2v-3"/></svg>"#,
        ),
        (
            "layers",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12.83 2.18a2 2 0 0 0-1.66 0L2.6 6.08a1 1 0 0 0 0 1.83l8.58 3.91a2 2 0 0 0 1.66 0l8.58-3.9a1 1 0 0 0 0-1.83z"/><path d="M2 12a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 12"/><path d="M2 17a1 1 0 0 0 .58.91l8.6 3.91a2 2 0 0 0 1.65 0l8.58-3.9A1 1 0 0 0 22 17"/></svg>"#,
        ),
        (
            "image",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>"#,
        ),
        (
            "lock",
            r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>"#,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use crate::icons::builtin;
    use crate::svg::parse_svg;

    #[test]
    fn every_builtin_parses() {
        for (name, svg) in builtin() {
            let icon = parse_svg(svg).unwrap_or_else(|e| panic!("icon {name}: {e}"));
            assert!(
                icon.viewbox[2] > 0.0 && icon.viewbox[3] > 0.0,
                "icon {name}: viewbox"
            );
            assert!(!icon.shapes.is_empty(), "icon {name}: no shapes");
        }
    }
}
