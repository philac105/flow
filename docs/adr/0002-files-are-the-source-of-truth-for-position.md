# Files are the source of truth for a Run's position

A Run's current Stage lives in `.flow/runs/<slug>.md` in the repo, not as a
label on the issue tracker — even when the tracker is GitHub and the labels
would render a board for free. The requirement Flow exists to serve is "I can
pick up where I left off, always", and a tracker label puts that answer behind
a network call, an API rate limit, and an auth token. Files are readable when
the session has died, the tokens are gone, and the machine is offline.

## Consequences

The tracker still holds the work items — specs and tickets stay on GitHub. Flow
holds only position and Handoff. Two stores means they can disagree; that
disagreement is named Drift and is surfaced rather than auto-resolved.
