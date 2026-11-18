# Security policy

## Reporting a problem

If you find a security problem in NomNom, please **do not open a public issue**. Report it privately instead:

1. Go to the [Security tab](https://github.com/Liveira/NomNom/security) of this repository.
2. Choose **Report a vulnerability**.
3. Say what you found, how to reproduce it, and which version and system you used.

You will get an answer as soon as I can give one. This is a small project run by one person, so please be patient. Once a fix is released I am happy to credit you, if you want.

## What counts

Things that matter most for NomNom:

- A way to make NomNom **overwrite, move or delete files it should not touch** (the organizer, converters, backup, disk analyzer, quick cleanup).
- Anything that lets a web page, another program or another user **talk to NomNom** without being allowed to (the browser extension, the native messaging host, the local pipe or socket).
- A way to get **NomNom to run a different program** than it should, or to pass it input that reaches a shell.
- A flaw in **update verification**, such as installing a file that does not match the published checksum or does not come from this repository's releases.
- Anything that makes NomNom **send your data somewhere** without you asking.

## Supported versions

Only the latest release gets fixes. The in-app update check (Settings → Updates) keeps you on it.

## How NomNom is built to limit harm

- It makes no network calls except downloads you ask for, tools you ask it to fetch, and the optional update check.
- Programs are started with an argument list, never through a shell.
- The browser connection uses a per-user pipe or socket, restricted to your account.
- Files are never replaced, destructive actions show a plan first, and the organizer keeps an undo journal.
