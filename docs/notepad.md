# Telephony Hello World (ESL control)

## Down to the basics
The main application loop will look like the following

`Connect → Subscribe → Command → Observe → Correlate → Control → Recover`

1. Connect: simply establish a connection between application and Freeswitch
2. Subscribe: application subscribes to Freeswitch events in PUB-SUB fashion
3. Command: in this first slice we will be using `originate`

When we send originate, we are saying:
“FreeSWITCH, create a new **call leg** with these parameters, give it this identity, and start executing it now.”

Key ideas packed into that:

### 1. We are creating a **call leg**
Not “a phone call”

A thing with lifecycle:
- created
- ringing
- answered
- bridged
- hung up

### 2. We are assigning it an identity (UUID)

We don’t want FreeSWITCH to pick an ID and tell us later.

We want to say: `This call is UUID = X`

So later:
- when events arrive
- when we hang it up
- when we correlate metrics

we know exactly which thing is which.

This is why originate includes: `{origination_uuid=...}`

### 3. We are starting execution, not waiting for a result
The we are setting up a PUB-SUB relationship between the application and Freeswitch


> Later, “originate” will be used to:
> - dial leads
> - dial agents
> - dial trunks
> - create whisper legs
> - create voicemail drops

## What is Event Socket Layer (ESL)?
ESL is basically the protocol glue between our application and Freeswitch

ESL gives we two channels:
1. **Command channel**

Our application says things like:
- originate
- uuid_kill
- uuid_bridge

This is imperative:
“Do this now.”

2. **Event channel**
FreeSWITCH emits things like:
- CHANNEL_CREATE
- CHANNEL_ANSWER
- CHANNEL_HANGUP

This is observational: “This happened.”

Our architecture must separate these two concerns.

> Main take aways:
>
> 1. Our application will command Freeswitch with the command channel
>
> 2. Our application subscribes to Freeswitch event channel in a PUB-SUB setup

## How will we use Freeswitch
FreeSWITCH is:
- A real-time call execution engine
- Extremely fast
- Event-driven
- Stateless from our business perspective

FreeSWITCH is not
- A workflow engine
- A source of business truth
- A place to encode dialer logic
- A reliable historian

Think of FreeSWITCH like: `exec() for phone calls`

We will:
- Tell it what to do
- Observe what happened
- Decide what to do next
