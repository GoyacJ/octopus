## Contract Change

- Change Type:
  - Internal Interface
- New / Updated Schemas:
  - None.
- New / Updated Commands:
  - New root scripts: `desktop:dev:local`, `remote-hub:dev`, `desktop:dev:remote`.
  - New desktop package scripts: `ui:dev`, `ui:build`, `tauri`, `dev`.
- New / Updated Queries:
  - None.
- New / Updated Events:
  - None.
- New / Updated DTOs:
  - None.
- Compatibility Impact:
  - Backward compatible for existing stable commands and shared contracts.
  - Adds a new app-local dev env interface: `OCTOPUS_REMOTE_HUB_DEV_SEED=1`.
- Affected Consumers:
  - Local developers using desktop or remote-hub dev workflows.
  - CI / local verification that asserts desktop config or script wiring.
- Migration Notes:
  - Stable workflows need no migration.
  - Dev users can switch from ad-hoc manual startup to the new explicit root commands.
- Generation Impact:
  - None.
- Open Questions:
  - None.

