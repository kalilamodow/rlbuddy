I'd advise you to read this if you're planning to go through the stats api service code. There's some funky stuff that goes on in it so I'd like to explain it here. Even if it's not super helpful hopefully it makes it a little easier to understand.

So, a quick rundown of how exactly the thing works:

1. Poll the tcp socket until we get a connection
2. Use `serde_json`'s stream deserializer which automatically gives us payloads whenever it finishes receiving a full json object (since tcp doesn't give us boundaries)
    - I am aware we can use the websockets version, but this avoids dependencies and I'm also too lazy.
3. Parse that into a `RawStatsApiEvent`. This is the actual stats api event serde struct!
4. Then, that turns into an `ApiEVent`. Since we also receive connection/disconnection events from the socket, we need that api event enum to distinguish between all that stuff.
5. If it's a connection/disconnection `ApiEvent` we just hand that off through RLEvent.

At this point, we now have a `RawStatsApiEvent` which we have to transform into an `RLEvent`! This is in `StatsApi::on_stats_api_event`. (sidenote: the big `match` is wrapped in a `Some`, which is why to return `None` we have to write the whole `return None` thing)

Most of them are simple, like `MatchDestroyed` -> `MatchLeft`, `GoalReplayStart` -> `ReplayStart` etc. The main weirdness is with the whole match creation/starting flow. This is what GameInitState is for. Basically, `RLEvent::MatchCreated/MatchStarted` also want to pass on a `MatchUpdate`. But, their raw stats api event equivalents (MatchCreated and the first CountdownStart) don't give all that stuff.

So, `StatsApi` has a `GameInitState` machine. here's an example scnario.

1. player is in main menu -> init state is `OutsideOfInit`
2. player joins a match -> stats api fires MatchCreated -> init state becomes `Created`
3. now, `UpdateState`s start flowing through. When it gets its first, the init state is `Created`. So, it returns `RLEvent::MatchCreated` and transitions to `GameInitState::AboutToStart`
4. omg `CountdownStart`! Since init is `AboutToStart`, now we switch it to `JustStarted`.
5. another `UpdateState` comes through, but init state is `JustStarted`! So instead of firing an `Update`, fire a `MatchStarted` with the stuff instead. And switch the init state to `OutsideOfInit`.
6. okay, more other `UpdateState`s, but they just go to `RLEvent::Update` since it's normal match time now.
