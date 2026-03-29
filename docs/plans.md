# Layerrd - Unfinished Tasks & Planned Work

## Stubbed-Out Features (UI exists but no implementation)

### File Operations
- [ ] **New Document** -- File > New menu item clicks but does nothing. Needs: dialog for canvas dimensions, reset document state.
- [ ] **Open Image** -- File > Open is stubbed. Needs: file dialog (rfd crate), image decoding (image crate), loading pixels into a new layer texture.
- [ ] **Save / Save As** -- Stubbed. Needs: project file format (serialize layer stack, blend modes, dimensions), file dialog.
- [ ] **Export** -- Stubbed. Needs: flatten to single image, encode as PNG/JPEG, file dialog.

### Edit Operations
- [ ] **Undo / Redo** -- Menu items exist but do nothing. Needs: command history stack, snapshot or diff-based state tracking.
- [ ] **Cut / Copy / Paste** -- Stubbed. Needs: selection system (see below), clipboard integration, GPU texture read-back.

### View Operations
- [ ] **Zoom In / Zoom Out** -- Stubbed. Currently the canvas auto-fits to the viewport with a fixed 0.9 scale factor. Needs: zoom state on document, scroll/pan support, keyboard shortcuts.
- [ ] **Fit to Window** -- Stubbed (though auto-fit is the current default behavior).

### Image Adjustments
- [ ] **Brightness/Contrast** -- GPU pipeline exists (`brightness_contrast_pipeline` in gpu.rs, shader in brightness_contrast.wgsl) but is **not wired into the UI**. Needs: dialog with sliders, apply to selected layer, preview support.
- [ ] **Resize Canvas** -- Stubbed. Needs: dialog, texture reallocation for all layers, data copy.
- [ ] **Rotate 90 CW/CCW** -- Stubbed. Needs: GPU shader or texture transform, update document dimensions.
- [ ] **Flip Horizontal / Vertical** -- Stubbed. Needs: GPU shader to flip pixel data.

### Layer Operations
- [ ] **Flatten Image** -- Stubbed. Needs: use existing composite result, replace layer stack with single layer.

## Tools (None Implemented)

All 8 tools (Select, Move, Brush, Eraser, Fill, Text, Eyedropper, Crop) are listed in the toolbar but **none have any canvas interaction logic**. The tool index is tracked (`selected_tool`) but never acted upon.

### Priority tool implementations:
- [ ] **Brush** -- Core drawing tool. Needs: mouse event handling on canvas, stroke rendering to layer texture (GPU or CPU), brush size/color/hardness settings.
- [ ] **Eraser** -- Similar to brush but writes transparent pixels.
- [ ] **Move** -- Drag layer content. Needs: per-layer offset/transform, updated compositing to respect offsets.
- [ ] **Select** -- Rectangular/freeform selection. Needs: selection mask, marching ants rendering, integration with cut/copy/paste.
- [ ] **Fill** -- Flood fill. Needs: GPU readback for seed pixel, fill algorithm (CPU or compute shader), write result.
- [ ] **Eyedropper** -- Color picker from canvas. Needs: GPU readback of pixel under cursor.
- [ ] **Text** -- Text rendering to layer. Needs: font rasterization, text input UI.
- [ ] **Crop** -- Resize canvas to selection. Needs: selection system, canvas resize.

## Missing Core Infrastructure

### Canvas Interaction
- [ ] **Mouse/pointer event handling on canvas** -- The central panel allocates a click_and_drag sense but events are not processed. Needs: coordinate mapping from screen space to canvas pixel space (accounting for zoom/pan).
- [ ] **Zoom and pan** -- No zoom/pan state exists. Needs: scroll wheel zoom, middle-click or space+drag pan, zoom level in status bar.

### Color System
- [ ] **Foreground/background color** -- No color state exists. Needs: color picker UI, color swatch in toolbar or properties panel.
- [ ] **Color picker dialog** -- Required for brush, fill, text tools.

### Selection System
- [ ] **Selection mask** -- No selection infrastructure. Needed for: cut/copy/paste, fill boundary, crop, selection-based edits.
- [ ] **Marching ants** -- Visual feedback for active selection on canvas.

### File I/O
- [ ] **Image decoding** -- Need `image` crate for loading PNG, JPEG, etc.
- [ ] **Image encoding** -- Need `image` crate for export/save.
- [ ] **File dialogs** -- Need `rfd` crate for native open/save dialogs.
- [ ] **Project format** -- Custom serialization for multi-layer documents (layer data, blend modes, opacity, names, dimensions).

### Performance & Polish
- [ ] **Keyboard shortcuts** -- No hotkeys are bound (Ctrl+Z for undo, B for brush, etc.).
- [ ] **Layer reordering** -- Layers can be added/deleted but not reordered via drag-and-drop.
- [ ] **Layer rename** -- No inline rename UI.
- [ ] **Resize layer textures** -- When canvas is resized, all layer textures need reallocation.
- [ ] **Async GPU readback** -- Needed for export, eyedropper, flood fill seed.

## Suggested Implementation Order

1. **Canvas interaction** (mouse events, coordinate mapping) -- unblocks all tools
2. **Color picker** -- unblocks brush, fill, text
3. **Brush tool** -- core editing capability
4. **Eraser tool** -- trivial once brush works
5. **Zoom/pan** -- critical UX
6. **Open/export images** (image + rfd crates) -- file I/O
7. **Undo/redo** -- essential for usable editing
8. **Wire up brightness/contrast** -- pipeline already exists
9. **Fill, eyedropper, move tools**
10. **Selection system + cut/copy/paste**
11. **Save/load project format**
12. **Remaining tools and polish** (text, crop, rotate, flip, keyboard shortcuts)
