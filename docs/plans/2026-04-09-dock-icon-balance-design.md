# Dock Icon Balance Design

**Goal:** Reduce the perceived size of the Octopus Dock icon while preserving the existing 3D octopus character, background color, and overall brand look.

**Context:** The current macOS Dock icon is bundled from the Tauri icon set under `apps/desktop/src-tauri/icons/*`. The icon appears visually larger than neighboring apps because the octopus subject expands close to the tile edges, especially through the outer tentacles.

## Approved Direction

- Preserve the current 3D octopus illustration.
- Preserve the rounded-square tile shape and existing light beige background.
- Reduce only the foreground subject scale to roughly `88%` of the current composition.
- Keep the octopus centered with the existing visual balance; do not redesign the pose or swap to the lobster reference.

## Rejected Alternatives

- Rebuild the icon to match `docs/lobster/logo.jpg`.
  Reason: changes the brand character and visual style too much.
- Shrink the whole icon tile instead of the octopus only.
  Reason: makes the app icon itself look undersized in Dock instead of correcting the subject balance inside the tile.
- Repose or redraw the tentacles.
  Reason: too much visual risk for a sizing problem.

## Implementation Notes

- Use the transparent root `logo.png` as the foreground source.
- Rebuild the icon master art with the existing rounded-square alpha shape.
- Regenerate the Tauri icon outputs from the updated master art.
- Verify the generated files exist at the expected Tauri paths and inspect the resulting master icon visually.
