# tunables.d schema

Each file = one UI tab/group. Files are loaded in filename order. Nothing here is a
hardcoded device assumption: every path is probed at runtime and entries whose node is
missing (or not writable) are hidden.

File:    { "group", "title", "sections":[ Section ] }
Section: { "id", "title", "foreach"?: Foreach, "entries":[ Entry ] }

Foreach (repeat a section per match):
  { "var":"policy", "glob":"/sys/devices/system/cpu/cpufreq/policy*",
    "order":"numeric", "titles":["Little cluster","Big cluster","Prime cluster"],
    "title_fallback":"Cluster {i}" }
  Placeholders usable in strings: {dir} (matched path), {name} (basename), {i} (rank, 0-based),
  {cur:<entry id suffix>} (current value of a sibling entry, e.g. {cur:governor}).
  Titles are assigned by RANK of the sorted matches, never by a fixed policy number.

Entry fields:
  id          unique within the section scope; persisted as "<scope>.<id>=<value>" in applied.conf
  label       UI label
  path        single node path
  paths       candidate list, first existing wins
  locate      {"names":[...],"roots":[...],"max_depth":N} search fallback; log misses to discover.log
  type        int | bool | enum | enum_bracket | freq | string | dir | glob_files | auto_glob | rgb | custom
  labels      value->text map (e.g. {"0":"Disabled","1":"Enabled"})
  zero_label  text shown when value is 0 
  choices_from node containing a space-separated list of options
  choices     static options
  min,max,step,scale   integers (NULL/absent when unset); scale divides raw value for display
  unit,display_unit    strings (NULL/absent when unset)
  risk                 "low" | "medium" | "high"; "high" nodes are confirmed by the UI before applying
  handler     named native handler in kmgr for multi-step writes
  bundle      entries sharing a bundle are applied together by the handler (one persisted record)
  write_order for paired nodes: "max_first_when_raising" | "min_first_when_lowering"
  verify      true = path/semantics not confirmed on this kernel, keep behind discovery + read-back check
  dir         type dir: expandable submenu listing every writable regular file under path
  glob_files  type glob_files: for each dir matching "glob", expose the listed "files"
  auto_glob   type auto_glob: expose every writable file matching roots/name_glob that is not
              already declared; value type inferred (int if numeric, else string)

Additions
  enum_int    int-valued enum: "values":[{"value":N,"label":"..."}], "unknown_label":"Unknown ({v})"
  apply       "once" (default) = write at boot only. Never re-assert unless the user enables hold.
  never_touch_mode_or_context   true = never chmod/chcon this node, never write it from post-fs-data
  optional    true = hide silently if no candidate exists (no discover.log noise)
  Files 05-sources.json and 80-features.json use "sources"/"sections" shapes: sources are read-only
  candidate lists for the monitor; features are ordinary entries.
