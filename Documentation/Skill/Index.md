# Skill corpus

The skills this repository ships, and the contract that keeps them loadable.

---

## The skills

- `Scene-State/` - what a decision model is told about the car's surroundings: why the dashboard's
  speed cannot say "blocked", the window that can, the sentence that has to travel with the numbers,
  and the escape that is a reverse *and* a turn.

---

## The contract

- One directory per skill, named in `PascalCase-Kebab`.
- `SKILL.md` carries the frontmatter and the rules; `Reference/` carries the worked instances, with
  `Reference/Index.md` listing them.
- Every rule is a claim with the observation it rests on, and that table ends the file.
- Nothing here names a machine, an account, a credential or a directory layout: this repository
  travels.
