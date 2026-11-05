chars: 3611
---
New
Computer
Automations
Artifacts
Plugins
Projects
Recents
Nikola Hristov
Add your team
Answer
Links
Images
Videos
Share
1:42 AM

How do SpacetimeDB reducers infer a component dependency graph and start order from self-registered rows, given reducers cannot run DDL?

Finished
2s

SpacetimeDB does not infer a component dependency graph or startup order from ordinary rows. The schema is declared statically when the module is compiled and published; reducers then operate only on data, so any dependency ordering must be represented explicitly in table rows and resolved by reducer logic.
docs
+1

What is static vs. runtime

Static: Table shapes, column types, indexes, constraints, reducers, and their generated bindings are registered from the module’s type definitions through SpacetimeDB’s self-describing type system and macros. This is effectively the “DDL” phase, and it happens at module build/publish time—not inside a reducer.
docs

Runtime: A reducer can insert, read, update, or delete rows, but it cannot create or alter a table’s schema. Reducers are transactional functions with read/write access to already-defined tables.
docs
+1

Lifecycle: The special init lifecycle reducer runs once when the module is first published or the database is cleared; it is the natural place to seed configuration or resolve an initial ordering stored in rows.
spacetimedb

How to encode dependencies

If “components” are domain entities—services, plugins, modules, workflow stages, etc.—model the graph as data:

rust
#[table(name = component, public)]
struct Component {
    #[primary_key]
    id: u64,
    name: String,
    depends_on: Vec<u64>, // IDs of prerequisite components
    status: ComponentStatus,
}

A reducer can then compute a valid start order from those rows, for example by topologically sorting components whose dependencies are already Ready. The graph is therefore application data, not database metadata.

Why reducers cannot “discover” it

Reducers only see the relational data available through ReducerContext; they cannot inspect or mutate the module’s schema, register new tables, or perform DDL. Consequently, there is no implicit mechanism by which inserting a Component row tells SpacetimeDB, “this component must start after that one.”
docs
+1

The intended pattern is:

Declare the component table and any status/config tables in the module schema.

Insert component rows through a reducer, migration seed, or init.

Have a reducer interpret depends_on relationships and transition statuses.

Use transactional reducer semantics so partial initialization is rolled back if ordering or validation fails.
docs
+1

Practical start-order pattern

A typical reducer might:

Load all components and their dependency IDs.

Reject cycles or missing dependencies.

Mark components with no unresolved dependencies as eligible.

Update their status and, if needed, schedule follow-up work through a scheduled reducer/table.

Repeat as dependencies become ready.

This keeps the dependency graph explicit, auditable, and transactional—rather than relying on SpacetimeDB to infer execution order from row insertion.
spacetimedb
+1

10 sources
Build a dependency graph and topological sort simulator for SpacetimeDB module initialization and self-registered rows
Computer
Compare SpacetimeDB module loading vs traditional SQL migration scripts for schema dependency and start-order resolution
Computer
Show me an example of a topological sort reducer
How does the init reducer handle startup sequences
Compare this pattern to Actor model lifecycles




Ask a follow-up
Search
Computer
Model
