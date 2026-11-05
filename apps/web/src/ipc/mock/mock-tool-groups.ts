/**
 * The MCP tool groups as `mcp_tool_groups` answers, mirrored from GROUPS in
 * the desktop crate's mcp/tools/groups.rs. A test holds the counts to the
 * tool-names snapshot, so a new tool shows up here too.
 */
import type { McpToolGroup } from '../client';

const row = (id: string, title: string, tools: number, about: string, alwaysOn = false) => ({
  id,
  title,
  about,
  tools,
  alwaysOn,
});

export const MOCK_TOOL_GROUPS: McpToolGroup[] = [
  row(
    'start',
    'Start',
    2,
    'get_started and create_project: where things stand and a first project',
    true
  ),
  row(
    'project',
    'Projects',
    9,
    'Projects, assets, the scene layout, copying assets and the asset trash',
    true
  ),
  row(
    'lab',
    'Lab',
    14,
    'Concept images and their jobs, the style board, picks, image backends and Codex usage'
  ),
  row(
    'refs',
    'Refs',
    5,
    "An asset's reference views: tagging, drawing missing ones, cleaning backgrounds"
  ),
  row(
    'plan',
    'Planning canvases',
    14,
    "Canvases and their nodes, the change log and a website's camera plan"
  ),
  row(
    'send',
    'Send to 3D',
    14,
    'Tripo, Meshy and by hand sends, redos, splits, credits, handoffs and downloads'
  ),
  row('inbox', 'Inbox', 7, 'Models waiting to be accepted: checks, accept, discard and restore'),
  row(
    'prepare',
    'Prepare',
    12,
    'Real size, fix-ups, parts, materials, assembling props and comparing versions'
  ),
  row('built', 'Built models', 3, 'Models a Blender script in the project makes, with no credits'),
  row(
    'pipeline',
    'Pipeline and ship',
    17,
    "Runs, presets, bake settings, starter scripts, shipping and the site's dev server"
  ),
  row('game', 'Game projects', 5, "A game project's repo, format, budgets, imports and ship plan"),
  row('library', 'Library', 12, 'Models kept outside any project, copied in and out'),
  row(
    'animation',
    'Animation',
    18,
    'The clip library, baking clips onto a rigged model and animation recipes'
  ),
  row('export', 'Export packs', 8, 'Export packs for other engines and render styles'),
  row('avatar-rig', 'Avatar and rig', 7, 'VRM avatars and the rig doctor'),
];
