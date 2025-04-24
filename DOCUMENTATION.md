# Qalendar API Documentation

## Table of Contents

- [Qalendar API Documentation](#qalendar-api-documentation)
  - [Table of Contents](#table-of-contents)
  - [Introduction](#introduction)
  - [Authentication](#authentication)
    - [Register User](#register-user)
    - [Login User](#login-user)
    - [Verify Email](#verify-email)
    - [Resend Verification Email](#resend-verification-email)
    - [Forgot Password](#forgot-password)
    - [Reset Password](#reset-password)
  - [Authenticated User ("Me") Endpoints](#authenticated-user-me-endpoints)
    - [Get My User Info](#get-my-user-info)
    - [Categories](#categories)
      - [Create Category](#create-category)
      - [List My Categories](#list-my-categories)
      - [Get Category by ID](#get-category-by-id)
      - [Update Category](#update-category)
      - [Delete Category (Soft)](#delete-category-soft)
    - [Deadlines](#deadlines)
      - [Create Deadline](#create-deadline)
      - [List My Deadlines](#list-my-deadlines)
      - [Get Deadline by ID](#get-deadline-by-id)
      - [Update Deadline](#update-deadline)
      - [Delete Deadline (Soft)](#delete-deadline-soft)
    - [Events](#events)
      - [Create Event](#create-event)
      - [List My Events](#list-my-events)
      - [Get Event by ID](#get-event-by-id)
      - [Update Event](#update-event)
      - [Delete Event (Soft)](#delete-event-soft)
    - [Event Invitations (Owner Actions)](#event-invitations-owner-actions)
      - [Invite User to Event](#invite-user-to-event)
      - [List Invitations for My Event](#list-invitations-for-my-event)
      - [Revoke Invitation](#revoke-invitation)
    - [Event Invitations (Invitee Actions)](#event-invitations-invitee-actions)
      - [List My Received Invitations](#list-my-received-invitations)
      - [Respond to Invitation](#respond-to-invitation)
    - [Calendar Shares (Owner Actions)](#calendar-shares-owner-actions)
      - [Create Calendar Share](#create-calendar-share)
      - [List My Created Shares](#list-my-created-shares)
      - [Get My Created Share by ID](#get-my-created-share-by-id)
      - [Update Calendar Share](#update-calendar-share)
      - [Delete Calendar Share (Soft)](#delete-calendar-share-soft)
  - [Calendar View Endpoints](#calendar-view-endpoints)
    - [Get My Consolidated Calendar](#get-my-consolidated-calendar)
    - [List Calendars Shared With Me](#list-calendars-shared-with-me)
    - [Get Specific Shared Calendar View](#get-specific-shared-calendar-view)
  - [Synchronization Endpoints](#synchronization-endpoints)
    - [Sync My Data](#sync-my-data)
    - [Sync Shared Calendar Data](#sync-shared-calendar-data)
  - [General Error Handling](#general-error-handling)
  - [Data Structures \& ENUMs](#data-structures--enums)

---

## Introduction

This document describes the RESTful API for the Qalendar application.

**Base URL:** The API endpoints are relative to the base URL where the server is running (e.g., `http://localhost:8000/api` or `http://api.qalendar.app/api`).

**Authentication:** Most endpoints require authentication using a JSON Web Token (JWT). Provide the token in the `Authorization` header as a Bearer token:
`Authorization: Bearer <your_jwt_token>`

**Data Format:** All request and response bodies use JSON (`Content-Type: application/json`). All timestamps are returned in **UTC** using the ISO 8601 / RFC 3339 format (e.g., `2023-10-27T10:30:00Z`).

**Soft Deletes:** Deleting items (Categories, Deadlines, Events, Shares, Invitations) typically performs a "soft delete" by setting a `deleted_at` timestamp. Sync endpoints will return these items with the `deleted_at` field set, allowing clients to remove them locally. Most `GET` and `UPDATE` endpoints will automatically ignore soft-deleted items unless otherwise specified.

---

## Authentication

Endpoints for user registration, login, verification, and password management. These endpoints generally do *not* require authentication.

### Register User

- **Purpose:** Creates a new user account and sends a verification email.
- **Method:** `POST`
- **Path:** `/auth/register`
- **Authentication:** None
- **Request Body:** (`RegisterUserPayload`)

    ```json
    {
      "displayName": "string (required, 1-100 chars)",
      "email": "string (required, valid email)",
      "password": "string (required, min 8 chars)",
      "dob": "string (optional, YYYY-MM-DD format)"
    }
    ```

- **Success Response:** `200 OK` (or `201 Created`) with `AuthResponse`

    ```json
    {
      "token": "string (JWT)",
      "user": {
        "userId": integer,
        "displayName": "string",
        "email": "string",
        "emailVerified": false,
        "createdAt": "string (ISO 8601 timestamp)",
        "dateOfBirth": "string (YYYY-MM-DD, optional)"
      }
    }
    ```

- **Error Responses:**
  - `400 Bad Request`: Validation failed (missing fields, invalid format). Body: `{"error": "Validation failed: ..."}`
  - `409 Conflict`: Email already in use. Body: `{"error": "Email address is already in use"}`
  - `500 Internal Server Error`: Database error, hashing error, email sending error. Body: `{"error": "..."}`

### Login User

- **Purpose:** Authenticates an existing user and returns a JWT.
- **Method:** `POST`
- **Path:** `/auth/login`
- **Authentication:** None
- **Request Body:** (`LoginUserPayload`)

    ```json
    {
      "email": "string (required, valid email)",
      "password": "string (required)"
    }
    ```

- **Success Response:** `200 OK` with `AuthResponse` (see Register User for structure). `emailVerified` will reflect the user's current status.

- **Error Responses:**
  - `400 Bad Request`: Validation failed. Body: `{"error": "Validation failed: ..."}`
  - `401 Unauthorized`: Invalid email or password, or user is soft-deleted. Body: `{"error": "Invalid email or password"}`
  - `403 Forbidden`: (Optional, if login requires verification) User not verified. Body: `{"error": "User email not verified"}`
  - `500 Internal Server Error`: Database error, hashing error. Body: `{"error": "..."}`

### Verify Email

- **Purpose:** Verifies a user's email address using the code sent during registration or resend.
- **Method:** `POST`
- **Path:** `/auth/verify-email`
- **Authentication:** None
- **Request Body:** (`VerifyEmailPayload`)

    ```json
    {
      "email": "string (required, valid email)",
      "code": "string (required, verification code)"
    }
    ```

- **Success Response:** `204 No Content`

- **Error Responses:**
  - `400 Bad Request`: Validation failed, invalid code, expired code. Body: `{"error": "..."}`
  - `404 Not Found`: User with that email not found. Body: `{"error": "User not found"}`
  - `409 Conflict`: User already verified. Body: `{"error": "User is already verified"}`
  - `500 Internal Server Error`: Database error. Body: `{"error": "..."}`

### Resend Verification Email

- **Purpose:** Generates a new verification code and resends the verification email to an unverified user.
- **Method:** `POST`
- **Path:** `/auth/resend-verification-email`
- **Authentication:** None
- **Request Body:** (`ResendVerificationEmailPayload`)

    ```json
    {
      "email": "string (required, valid email)"
    }
    ```

- **Success Response:** `204 No Content`

- **Error Responses:**
  - `400 Bad Request`: Validation failed. Body: `{"error": "..."}`
  - `404 Not Found`: User with that email not found. Body: `{"error": "User not found"}`
  - `409 Conflict`: User already verified. Body: `{"error": "User is already verified"}`
  - `500 Internal Server Error`: Database error, email sending error. Body: `{"error": "..."}`

### Forgot Password

- **Purpose:** Sends a password reset code to the user's email address if the user exists.
- **Method:** `POST`
- **Path:** `/auth/forgot-password`
- **Authentication:** None
- **Request Body:** (`ForgotPasswordPayload`)

    ```json
    {
      "email": "string (required, valid email)"
    }
    ```

- **Success Response:** `204 No Content` (Returned even if email doesn't exist to prevent email enumeration).

- **Error Responses:**
  - `400 Bad Request`: Validation failed. Body: `{"error": "..."}`
  - `403 Forbidden`: (Optional, if reset requires verification) User not verified. Body: `{"error": "User email not verified"}`
  - `500 Internal Server Error`: Database error, email sending error. Body: `{"error": "..."}`

### Reset Password

- **Purpose:** Sets a new password for the user using the code sent via the forgot password email.
- **Method:** `POST`
- **Path:** `/auth/reset-password`
- **Authentication:** None
- **Request Body:** (`ResetPasswordPayload`)

    ```json
    {
      "email": "string (required, valid email)",
      "code": "string (required, reset code)",
      "newPassword": "string (required, min 8 chars)"
    }
    ```

- **Success Response:** `204 No Content`

- **Error Responses:**
  - `400 Bad Request`: Validation failed, invalid/expired code, password too short. Body: `{"error": "..."}`
  - `500 Internal Server Error`: Database error, hashing error. Body: `{"error": "..."}`

---

## Authenticated User ("Me") Endpoints

Endpoints related to the currently authenticated user's data and actions.

**Authentication:** All endpoints in this section require a valid `Authorization: Bearer <token>` header.

### Get My User Info

- **Purpose:** Retrieves basic information about the authenticated user.
- **Method:** `GET`
- **Path:** `/me`
- **Success Response:** `200 OK`

    ```json
    {
      "message": "You are authenticated!",
      "userId": integer
    }
    ```

- **Error Responses:**
  - `401 Unauthorized`: Invalid or missing token.

### Categories

Endpoints for managing the user's own categories (`/api/me/categories`).

#### Create Category

- **Method:** `POST`
- **Path:** `/me/categories`
- **Request Body:** (`CreateCategoryPayload`)

    ```json
    {
      "name": "string (required, 1-255 chars)",
      "color": "string (required, hex format #RGB or #RRGGBB)"
    }
    ```

- **Success Response:** `201 Created` with the created `Category` object (see [Data Structures](#data-structures--enums)).

- **Error Responses:** `400`, `401`, `409` (Unique constraint `user_id, name` violation), `500`.

#### List My Categories

- **Method:** `GET`
- **Path:** `/me/categories`
- **Success Response:** `200 OK` with an array of `Category` objects belonging to the user. `[]` if none.
- **Error Responses:** `401`, `500`.

#### Get Category by ID

- **Method:** `GET`
- **Path:** `/me/categories/{category_id}`
- **Path Parameters:**
  - `category_id` (integer): The ID of the category to retrieve.
- **Success Response:** `200 OK` with the specified `Category` object.
- **Error Responses:** `401`, `404` (Not found or doesn't belong to user), `500`.

#### Update Category

- **Method:** `PUT`
- **Path:** `/me/categories/{category_id}`
- **Path Parameters:**
  - `category_id` (integer): The ID of the category to update.
- **Request Body:** (`UpdateCategoryPayload`) - Send only fields to update.

    ```json
    {
      "name": "string (optional, 1-255 chars)",
      "color": "string (optional, hex format)",
    }
    ```

- **Success Response:** `200 OK` with the updated `Category` object.

- **Error Responses:** `400`, `401`, `404`, `409` (Unique name constraint), `500`.

#### Delete Category (Soft)

- **Method:** `DELETE`
- **Path:** `/me/categories/{category_id}`
- **Path Parameters:**
  - `category_id` (integer): The ID of the category to delete.
- **Success Response:** `204 No Content`
- **Error Responses:** `401`, `404` (Not found or doesn't belong to user), `500`.

### Deadlines

Endpoints for managing the user's own deadlines (`/api/me/deadlines`).

#### Create Deadline

- **Method:** `POST`
- **Path:** `/me/deadlines`
- **Request Body:** (`CreateDeadlinePayload`)

    ```json
    {
      "title": "string (required, 1-255 chars)",
      "categoryId": integer (optional, must exist and belong to user if provided),
      "description": "string (optional, max 1000 chars)",
      "dueDate": "string (required, ISO 8601 format, e.g., 2023-11-15T14:00:00Z)",
      "priority": "string (optional, 'normal' | 'important' | 'urgent', defaults to 'normal')",
      "workloadMagnitude": integer (optional, required if workloadUnit present),
      "workloadUnit": "string (optional, 'minutes' | 'hours' | 'days', required if workloadMagnitude present)"
    }
    ```

- **Success Response:** `201 Created` with the created `Deadline` object (see [Data Structures](#data-structures--enums)).

- **Error Responses:** `400` (Validation, invalid categoryId), `401`, `500`.

#### List My Deadlines

- **Method:** `GET`
- **Path:** `/me/deadlines`
- **Success Response:** `200 OK` with an array of `Deadline` objects belonging to the user. `[]` if none.
- **Error Responses:** `401`, `500`.

#### Get Deadline by ID

- **Method:** `GET`
- **Path:** `/me/deadlines/{deadline_id}`
- **Path Parameters:**
  - `deadline_id` (integer): The ID of the deadline to retrieve.
- **Success Response:** `200 OK` with the specified `Deadline` object.
- **Error Responses:** `401`, `404` (Not found or doesn't belong to user), `500`.

#### Update Deadline

- **Method:** `PUT`
- **Path:** `/me/deadlines/{deadline_id}`
- **Path Parameters:**
  - `deadline_id` (integer): The ID of the deadline to update.
- **Request Body:** (`UpdateDeadlinePayload`) - Send only fields to update. Explicitly send `"field": null` to clear optional fields like `categoryId`, `description`, `workloadMagnitude`, `workloadUnit`.

    ```json
    {
      "title": "string (optional, 1-255 chars)",
      "categoryId": integer | null (optional),
      "description": "string | null (optional)",
      "dueDate": "string (optional, ISO 8601 format)",
      "priority": "string (optional, 'normal' | 'important' | 'urgent')",
      "workloadMagnitude": integer | null (optional, must be paired with unit or both null),
      "workloadUnit": "string | null (optional, 'minutes' | 'hours' | 'days', must be paired with magnitude or both null)"
    }
    ```

- **Success Response:** `200 OK` with the updated `Deadline` object.

- **Error Responses:** `400` (Validation, invalid categoryId), `401`, `404`, `500`.

#### Delete Deadline (Soft)

- **Method:** `DELETE`
- **Path:** `/me/deadlines/{deadline_id}`
- **Path Parameters:**
  - `deadline_id` (integer): The ID of the deadline to delete.
- **Success Response:** `204 No Content`
- **Error Responses:** `401`, `404`, `500`.

### Events

Endpoints for managing the user's own base event records (`/api/me/events`). Occurrences are not managed here.

#### Create Event

- **Method:** `POST`
- **Path:** `/me/events`
- **Request Body:** (`CreateEventPayload`)

    ```json
    {
      "title": "string (required, 1-255 chars)",
      "categoryId": integer (optional, must exist and belong to user if provided),
      "description": "string (optional, max 1000 chars)",
      "startTime": "string (required, ISO 8601 format)",
      "endTime": "string (required, ISO 8601 format)",
      "location": "string (optional, max 255 chars)",
      "rrule": "string (optional, iCalendar RRULE format)"
    }
    ```

- **Success Response:** `201 Created` with the created `Event` object (see [Data Structures](#data-structures--enums)).

- **Error Responses:** `400` (Validation, invalid categoryId), `401`, `500`.

#### List My Events

- **Method:** `GET`
- **Path:** `/me/events`
- **Success Response:** `200 OK` with an array of base `Event` objects belonging to the user. `[]` if none.
- **Error Responses:** `401`, `500`.

#### Get Event by ID

- **Method:** `GET`
- **Path:** `/me/events/{event_id}`
- **Path Parameters:**
  - `event_id` (integer): The ID of the event to retrieve.
- **Success Response:** `200 OK` with the specified base `Event` object.
- **Error Responses:** `401`, `404` (Not found or doesn't belong to user), `500`.

#### Update Event

- **Method:** `PUT`
- **Path:** `/me/events/{event_id}`
- **Path Parameters:**
  - `event_id` (integer): The ID of the event to update.
- **Request Body:** (`UpdateEventPayload`) - Send only fields to update. Send `"field": null` to clear optional fields.

    ```json
    {
      "title": "string (optional, 1-255 chars)",
      "categoryId": integer | null (optional),
      "description": "string | null (optional)",
      "startTime": "string (optional, ISO 8601 format)",
      "endTime": "string (optional, ISO 8601 format)",
      "location": "string | null (optional)",
      "rrule": "string | null (optional)"
    }
    ```

- **Success Response:** `200 OK` with the updated base `Event` object.

- **Error Responses:** `400` (Validation, invalid categoryId), `401`, `404`, `500`.

#### Delete Event (Soft)

- **Method:** `DELETE`
- **Path:** `/me/events/{event_id}`
- **Path Parameters:**
  - `event_id` (integer): The ID of the event to delete.
- **Success Response:** `204 No Content`
- **Error Responses:** `401`, `404`, `500`.

### Event Invitations (Owner Actions)

Endpoints for the owner of an event to manage invitations (`/api/me/events/{event_id}/invitations`).

#### Invite User to Event

- **Method:** `POST`
- **Path:** `/me/events/{event_id}/invitations`
- **Path Parameters:**
  - `event_id` (integer): The ID of the event to invite to (must be owned by user).
- **Request Body:** (`InviteUserPayload`)

    ```json
    {
      "invitedUserEmail": "string (required, valid email)"
    }
    ```

- **Success Response:** `201 Created` with the created `EventInvitation` object.

- **Error Responses:** `400` (Validation), `401`, `404` (Event not found or not owned by user, Invited user not found), `500`.

#### List Invitations for My Event

- **Method:** `GET`
- **Path:** `/me/events/{event_id}/invitations`
- **Path Parameters:**
  - `event_id` (integer): The ID of the event (must be owned by user).
- **Query Parameters:**
  - `status` (string, optional): Filter by status (`pending`, `accepted`, `rejected`, `maybe`).
- **Success Response:** `200 OK` with an array of `EventInvitationResponseItem` objects (includes invited user details). `[]` if none.
- **Error Responses:** `401`, `404` (Event not found or not owned by user), `500`.

#### Revoke Invitation

- **Method:** `DELETE`
- **Path:** `/me/events/{event_id}/invitations/{invitation_id}`
- **Path Parameters:**
  - `event_id` (integer): The ID of the event (must be owned by user).
  - `invitation_id` (integer): The ID of the invitation to revoke.
- **Success Response:** `204 No Content`
- **Error Responses:** `401`, `404` (Event not found or not owned, Invitation not found or not for this event), `500`.

### Event Invitations (Invitee Actions)

Endpoints for users to manage invitations they have received (`/api/me/invitations`).

#### List My Received Invitations

- **Method:** `GET`
- **Path:** `/me/invitations`
- **Query Parameters:**
  - `status` (string, optional): Filter by status (`pending`, `accepted`, `rejected`, `maybe`).
- **Success Response:** `200 OK` with an array of `MyInvitationResponseItem` objects (includes event details). `[]` if none.
- **Error Responses:** `401`, `500`.

#### Respond to Invitation

- **Method:** `PUT`
- **Path:** `/me/invitations/{invitation_id}/status`
- **Path Parameters:**
  - `invitation_id` (integer): The ID of the invitation to respond to (must be for the authenticated user).
- **Request Body:** (`InvitationResponsePayload`)

    ```json
    {
      "status": "string (required, 'accepted' | 'rejected' | 'maybe')"
    }
    ```

- **Success Response:** `200 OK` with the updated base `EventInvitation` object.

- **Error Responses:** `400` (Validation, invalid status), `401`, `404` (Invitation not found or not for this user), `500`.

### Calendar Shares (Owner Actions)

Endpoints for the owner to manage calendar shares they created (`/api/me/shares`).

#### Create Calendar Share

- **Method:** `POST`
- **Path:** `/me/shares`
- **Request Body:** (`CreateSharePayload`)

    ```json
    {
      "sharedWithUserEmail": "string (required, valid email)",
      "categoryIds": [integer] (required, array of category IDs, min 1, must exist and belong to user),
      "message": "string (optional, max 1000 chars)",
      "privacyLevel": "string (optional, 'fullDetails' | 'busyOnly', defaults to 'fullDetails')",
      "expiresAt": "string (optional, ISO 8601 format)"
    }
    ```

- **Success Response:** `201 Created` with the created `ShareDetailsResponse` object (includes shared_with user details and category IDs).

- **Error Responses:** `400` (Validation, invalid categoryIds), `401`, `404` (sharedWithUser not found), `409` (Share already exists with this user), `500`.

#### List My Created Shares

- **Method:** `GET`
- **Path:** `/me/shares`
- **Success Response:** `200 OK` with an array of `ListSharesResponseItem` objects created by the user. `[]` if none.
- **Error Responses:** `401`, `500`.

#### Get My Created Share by ID

- **Method:** `GET`
- **Path:** `/me/shares/{share_id}`
- **Path Parameters:**
  - `share_id` (integer): The ID of the share to retrieve (must be owned by user).
- **Success Response:** `200 OK` with the specified `ShareDetailsResponse` object.
- **Error Responses:** `401`, `404` (Not found or doesn't belong to user), `500`.

#### Update Calendar Share

- **Method:** `PUT`
- **Path:** `/me/shares/{share_id}`
- **Path Parameters:**
  - `share_id` (integer): The ID of the share to update (must be owned by user).
- **Request Body:** (`UpdateSharePayload`) - Send only fields to update. Send `"field": null` to clear optional fields like `message`, `expiresAt`. Send `categoryIds: []` to remove all categories.

    ```json
    {
      "categoryIds": [integer] (optional, array of category IDs, must exist and belong to user),
      "message": "string | null (optional)",
      "privacyLevel": "string (optional, 'fullDetails' | 'busyOnly')",
      "expiresAt": "string | null (optional, ISO 8601 format)"
    }
    ```

- **Success Response:** `200 OK` with the updated `ShareDetailsResponse` object.

- **Error Responses:** `400` (Validation, invalid categoryIds), `401`, `404`, `500`.

#### Delete Calendar Share (Soft)

- **Method:** `DELETE`
- **Path:** `/me/shares/{share_id}`
- **Path Parameters:**
  - `share_id` (integer): The ID of the share to delete (must be owned by user).
- **Success Response:** `204 No Content`
- **Error Responses:** `401`, `404`, `500`.

---

## Calendar View Endpoints

Endpoints for viewing combined calendar data.

**Authentication:** All endpoints in this section require a valid `Authorization: Bearer <token>` header.

### Get My Consolidated Calendar

- **Purpose:** Retrieves all relevant calendar items (owned events, owned deadlines, accepted invites) for the authenticated user. Does *not* perform recurrence expansion or range filtering (frontend responsibility).
- **Method:** `GET`
- **Path:** `/calendar`
- **Success Response:** `200 OK` with `UserCalendarResponse` object.

    ```json
    {
      "events": [Event], // Array of base Event objects
      "deadlines": [Deadline] // Array of Deadline objects
    }
    ```

- **Error Responses:** `401`, `500`.

### List Calendars Shared With Me

- **Purpose:** Retrieves a list of calendar shares where the authenticated user is the recipient.
- **Method:** `GET`
- **Path:** `/calendar/shares`
- **Success Response:** `200 OK` with an array of `ListSharesResponseItem` objects (includes owner details and shared category IDs). `[]` if none.
- **Error Responses:** `401`, `500`.

### Get Specific Shared Calendar View

- **Purpose:** Retrieves calendar items (events, deadlines) from a specific shared calendar, applying category filters and privacy rules set by the owner. Does *not* perform recurrence expansion or range filtering (frontend responsibility).
- **Method:** `GET`
- **Path:** `/calendar/shares/{share_id}`
- **Path Parameters:**
  - `share_id` (integer): The ID of the share instance to view (must be shared with the authenticated user).
- **Success Response:** `200 OK` with `SharedCalendarResponse` object.

    ```json
    {
      "shareInfo": CalendarShare, // Details of the share instance
      "events": [Event], // Array of Event objects (details masked if privacy='busyOnly')
      "deadlines": [Deadline] // Array of Deadline objects (details masked if privacy='busyOnly')
    }
    ```

  - **Note on `busyOnly`:** If `shareInfo.privacyLevel` is `busyOnly`, event/deadline `title`, `description`, `location`, `category_id` will be masked/nulled, and `priority` will be reset to `normal`.
- **Error Responses:** `401`, `404` (Share not found, not shared with user, or expired), `500`.

---

## Synchronization Endpoints

Endpoints for efficiently syncing data between the client and server.

**Authentication:** All endpoints in this section require a valid `Authorization: Bearer <token>` header.

### Sync My Data

- **Purpose:** Retrieves all owned/relevant items (categories, deadlines, events, invitations, shares) that have been created, updated, or soft-deleted since a given timestamp.
- **Method:** `GET`
- **Path:** `/sync/me`
- **Query Parameters:**
  - `since` (string, optional): ISO 8601 timestamp. If provided, only returns items updated *after* this time. If omitted, returns all accessible items.
- **Success Response:** `200 OK` with `SyncResponse` object.

    ```json
    {
      "categories": [Category], // Includes soft-deleted (check deleted_at)
      "deadlines": [Deadline], // Includes soft-deleted
      "events": [Event], // Includes owned & accepted invites updated since 'since', includes soft-deleted
      "receivedInvitations": [EventInvitation], // Includes soft-deleted
      "sharesCreated": [ListSharesResponseItem], // Includes soft-deleted
      "sharesReceived": [ListSharesResponseItem], // Includes soft-deleted
      "syncTimestamp": "string (ISO 8601 timestamp of sync)" // Use this for next 'since' param
    }
    ```

  - **Client Handling:** Client should process each array, updating existing items by ID, adding new items, and removing items where `deleted_at` is not null. Store the `syncTimestamp` for the next request.
- **Error Responses:** `400` (Invalid `since` format), `401`, `500`.

### Sync Shared Calendar Data

- **Purpose:** Retrieves updates to a specific shared calendar view (share config, events, deadlines) since a given timestamp, respecting filters and privacy.
- **Method:** `GET`
- **Path:** `/sync/calendar/shares/{share_id}`
- **Path Parameters:**
  - `share_id` (integer): The ID of the share instance to sync.
- **Query Parameters:**
  - `since` (string, optional): ISO 8601 timestamp.
- **Success Response:** `200 OK` with `SyncSharedCalendarResponse` object.

    ```json
    {
      // Share info is only present if the share itself was updated or if items were updated since 'since'
      "shareInfo": CalendarShare | null, // Includes deleted_at if share was revoked
      "events": [Event], // Updated events (privacy applied), includes soft-deleted
      "deadlines": [Deadline], // Updated deadlines (privacy applied), includes soft-deleted
      "syncTimestamp": "string (ISO 8601 timestamp of sync)"
    }
    ```

  - **Client Handling:** If `shareInfo` is present and has `deleted_at` set, remove the shared calendar view. Otherwise, update local `shareInfo` if present. Process `events` and `deadlines` arrays like in `/sync/me`, applying privacy rules if needed (though the API should already have applied them). Store `syncTimestamp`. If `shareInfo` is `null` and `since` was provided, it might mean the share is no longer accessible or no relevant updates occurred.
- **Error Responses:** `400` (Invalid `since` format), `401`, `404` (Share not found or not accessible), `500`.

---

## General Error Handling

Errors are generally returned with an appropriate HTTP status code (4xx for client errors, 5xx for server errors) and a JSON body containing an error message:

```json
{
  "error": "A descriptive error message"
}
```

Common status codes include:

- `400 Bad Request`: Invalid input format, validation errors.
- `401 Unauthorized`: Missing or invalid JWT token.
- `403 Forbidden`: Authenticated user lacks permission for the action (e.g., email not verified).
- `404 Not Found`: Resource not found or user lacks access to it.
- `409 Conflict`: Resource creation conflict (e.g., email already exists).
- `500 Internal Server Error`: Unexpected server-side error (database issue, coding error, etc.). Check server logs.

---

## Data Structures & ENUMs

*(Refer to the Rust model definitions (`src/models/*.rs`) for the exact structure of response objects like `Category`, `Deadline`, `Event`, `EventInvitation`, `CalendarShare`, etc. Ensure frontend models match the `#[serde(rename_all = "camelCase")]` convention used.)*

**Key ENUMs:**

- `DeadlinePriorityLevel`: `"normal"`, `"important"`, `"urgent"`
- `WorkloadUnitType`: `"minutes"`, `"hours"`, `"days"`
- `EventInvitationStatus`: `"pending"`, `"accepted"`, `"rejected"`, `"maybe"`
- `SharePrivacyLevel`: `"fullDetails"`, `"busyOnly"`
