# Testing

## General rule for testing
A quick rule we can follow

If our testing behavior across multiple collaborators (telephony + clock + persistence + metrics): use trait objects in the app/service layer.

If our testing pure decision logic with minimal dependencies: use generics (or even pure functions)