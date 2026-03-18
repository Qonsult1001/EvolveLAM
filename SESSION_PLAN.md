## Session Plan

### Task 1: Fix evolve-ide.sh SESSION_START_SHA when finish runs without session metadata
Files: scripts/evolve-ide.sh
Description: When finish runs without having run setup in this session (or .evolve/metadata.sh is missing), SESSION_START_SHA is never set and the script hits "unbound variable" with set -u. After load_metadata in phase_finish, set SESSION_START_SHA="${SESSION_START_SHA:-$(git rev-parse HEAD)}" so the variable is always defined; use HEAD as the session start when no metadata exists. Add the same default after load_metadata in any other phase that uses SESSION_START_SHA, or set it once at top of phase_finish after load_metadata.
Issue: none

### Issue Responses
- No community issues today.
