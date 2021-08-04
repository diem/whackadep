### libc change

libc changed to 0.2.92 to 0.2.93
which contains change in build.rs file

### guppy change

guppy changed from 0.8.0 to 0.9.0
which changes guppy (target)
and cargo-metadata (host)

### conflict change

The goal is to inform devs
on multiple copies of different versions of dependencies
that may result in namespace conflict.

##### Case 1:

Cargo.toml initially decalred guppy=0.7.0 and target-spec=0.6.0.
Updating to guppy=0.9.0 results in target-spec=0.7.0 to be pulled in as the new version depends on target-spec=0.7.0.
Therefore, we have target-spec=0.6.0 as a direct dep
and target-spec=0.7.0 as a transitive dep to guppy=0.9.0
which may create some namespace conflict if
there is such breaking changes.

### rustsec change

updated tokio from 1.7.1 to 1.7.2. 1.7.1 has RUSTSEC-2016-0005.

### depkind metadata

changed valid_dep to include deps in all dep kind section -
normal, build, dev
