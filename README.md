# An email newsletter API, in Rust
## Overview
This repo implements the basic functions of an email newsletter service, which allows authenticated users to send newsletters to their subscribers.

This repo is a project for **learning purposes** (primarily, production Rust and ecosystem crates) and was guided by the Zero to Production (ZtP) resource authored by Luca Palmieri.
The basic features guided by the ZtP, as well as the additional features I implemented myself for learning are noted below.

## Features

### Basic features
- Allow users to subscribe to a newsletter and confirm their subscription via a link sent to the user's email. Includes email validation with the validator crate and tests with the fake crate
- Logging of API endpoints and inner functions with the tracing crate
- Author login portal with session-based authentication and a change password function
- An open send newsletter API endpoint, which allows anyone to send newsletters. The endpoint uses the [Postmark API](https://postmarkapp.com/)
- Idempotent newsletter sending, which handles duplicate requests to send a newsletter (For example, an author clicking the send button twice)
- A background worker to deliver newsletter issues from the newsletter delivery queue

### Additional, self-implemented features
- Handle duplicate subscribe actions by the same email address
  - If already confirmed through link, return HTTP 200
  - If not yet confirmed, generate a new subscription token and send a new confirmation link
- Gate the sending of newsletters behind user authentication using session-based authentication. Only allow authenticated authors to send newsletter issues
  - Uses a function converted to middleware (by the actix-web-lab crate) to determine if a valid session exists (or redirects to login)
  - In the send newsletter POST handler, uses Form extractor from actix_web (application/x-www-form-urlencoded) to handle the send newsletter request body (title, html content, text content, and a generated idempotency key)
- Add validation to change password function to ensure proper length (between 12 and 128 characters)

## Tech Stack
### Actix-web - Rust web framework crate
- Middleware: TracingLogger from tracing crate, FlashMessagesFramework (signed cookies) from actix_web_flash_messages, SessionMiddleware (create & store sessions) from actix_session
- Application state: Postgres connection pool, email client (API token, sender email), app url, HMAC secret for signed message cookies and session cookies

### Postgres - Data Model:
-   Subscribers table which stores a name, email, subscriber id, time, and subscription status,
-   Subscription tokens table which stores a subscription token and subscriber id mapped to subscribers to enable confirmations
-   Authorized users table which stores a username and an Argon2Id password hash
-   Idempotency table which stores an idempotency key and an HTTP response
-   Newsletter table which stores a newsletter issue id, title, html and text contents, and a published date 
-   Newsletter delivery queue which stores a newsletter issue ID and a subscriber email as primary keys to manage newsletter delivery workflow

### Redis
- Stores session token as key and session state as JSON value (user id is the session state)

## Deployment
Deployed to Digital Ocean through a Docker image and via spec.yaml configuration, with a provisioned Postgres and Redis instance
