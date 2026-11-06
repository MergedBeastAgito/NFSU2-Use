# Skill corpus

The skills this repository ships, and the contract that keeps them loadable.

---

## The skills

- `Scene-State/` - what a decision model is told about the car's surroundings:
  why the dashboard's speed cannot say "blocked", the window that can, the
  sentence that has to travel with the numbers, and the escape that is a reverse
  _and_ a turn.

---

## The contract

- One directory per skill, named in `PascalCase-Kebab`, holding a `SKILL.md`
  whose frontmatter states a non-empty `description` and whose `name` matches
  the directory.
- `SKILL.md` carries the frontmatter and the rules; `Reference/` carries the
  worked instances that outgrow a rule, and `Reference/Index.md` names every
  file in that directory.
- Every rule is a claim with the observation it rests on, and that table ends
  the file.
- Nothing here names a machine, an account, a credential or a directory layout:
  this repository travels.
