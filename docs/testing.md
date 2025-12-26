# Testing

## General rule for testing
A quick rule you can follow

If you’re testing behavior across multiple collaborators (telephony + clock + persistence + metrics): use trait objects in the app/service layer.

If you’re testing pure decision logic with minimal dependencies: use generics (or even pure functions)