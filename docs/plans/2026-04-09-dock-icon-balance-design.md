# Dock Icon Balance Design

**Goal:** Reduce the perceived size of the Octopus Dock icon while preserving the existing 3D octopus character, background color, and overall brand look.

**Context:** The current macOS Dock icon is bundled from the Tauri icon set under `apps/desktop/src-tauri/icons/*`. The icon appears visually larger than neighboring apps because the full rounded-square tile occupies too much of the icon canvas, so the Dock reads the entire icon silhouette as oversized even when the octopus subject itself is reduced.

## Approved Direction

- Preserve the current 3D octopus illustration.
- Preserve the rounded-square tile shape and existing light beige background.
- Reduce the scale of the full rounded-square tile to roughly `88%` of the current canvas so the outer silhouette gains meaningful transparent padding.
- Preserve the current centered composition inside the tile; do not redesign the pose or swap to the lobster reference.

## Rejected Alternatives

- Rebuild the icon to match `docs/lobster/logo.jpg`.
  Reason: changes the brand character and visual style too much.
- Reduce only the foreground subject while keeping the tile at nearly full canvas size.
  Reason: the Dock still reads the large outer tile silhouette as oversized, so the perceived size barely changes.
- Repose or redraw the tentacles.
  Reason: too much visual risk for a sizing problem.

## Implementation Notes

- Use the current desktop master icon as the source composition.
- Rebuild the master icon by scaling the full tile inside a transparent `1024x1024` canvas.
- Regenerate the Tauri icon outputs from the updated master art.
- Verify the generated files exist at the expected Tauri paths and inspect the resulting master icon visually.
