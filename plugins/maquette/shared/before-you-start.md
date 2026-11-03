## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list; it is short, and `detail: true` gives every row and gate. No project to work in: `create_project` (`target` is `website` or `game`) makes one and spends nothing. No tool removes a project; a removed project's id fails with not-found, and only a person restores it. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.
Settings > Agents can turn tool groups off, all but start and project: their tools are missing from
your list (or answer code `group-off`), and `get_started` names them in `toolGroupsOff`. Any tool this
skill names may be off. When a step needs a missing tool, do not work around it: ask the person to
turn its group on in Settings > Agents, then go on.
A project's planning canvases (the plan group) hold the person's notes, links and plans: `list_canvases`
lists them and `get_canvas` reads one, so read the canvas the person points at before you plan for them.
Agents never delete a person's nodes: ask with `propose_removal`, and only the person answers.
A website project's camera plan (`get_camera_plan`) is tried there until the person presses Apply
in the app: agents may propose a tour, never apply one.
