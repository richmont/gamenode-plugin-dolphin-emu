# Contributor instructions

Follow CONTRIBUTING.md. Use only the public GameNode SDK; the host owns emulator execution.
Planning completion does not establish implementation or native acceptance.

## CodeGraph

This repository has a local `.codegraph/` index. Before searching for or reading code to locate
or understand symbols, use `codegraph explore "<question or symbols>"` or the CodeGraph MCP
`codegraph_explore` tool with this repository's project path. Use `codegraph sync` after source
changes and `codegraph status` to inspect the index. Keep generated index data out of Git.

## OpenSpec

Project context is in `openspec/config.yaml`; active plans are in `openspec/changes/`.
Use the generated `.agents/skills/openspec-*` workflows where applicable. Validate changes with
`openspec validate --all --strict --no-interactive`. Keep root documentation and the active
change's tasks consistent, and leave implementation tasks unchecked until evidence exists.
