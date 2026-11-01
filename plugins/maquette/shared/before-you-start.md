## Before you start

Call `get_started` first. It says whether a workspace is open, which tools work, which keys are saved
(true or false, never a value), every project with its stage, the gates only a person can pass, and
a `next` list. No project to work in: `create_project` makes one and spends nothing. If the Maquette
tools fail with `ECONNREFUSED`, the app is closed or its server is off: ask the person to open
Maquette and turn the server on in Settings > Agents. An error names its `code`, and `fix` and
`tryTool` when they help; `personOnly` means ask the person instead of trying again.
