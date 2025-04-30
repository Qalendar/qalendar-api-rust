-- Add the UUID extension if it's not already enabled in your database
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Drop types and tables in reverse order of dependency if they exist
DROP TABLE IF EXISTS event_exceptions CASCADE;
DROP TABLE IF EXISTS event_invitations CASCADE;
DROP TABLE IF EXISTS calendar_share_categories CASCADE;
DROP TABLE IF EXISTS calendar_shares CASCADE;
DROP TABLE IF EXISTS open_calendar_share_categories CASCADE;
DROP TABLE IF EXISTS open_calendar_shares CASCADE;
DROP TABLE IF EXISTS events CASCADE;
DROP TABLE IF EXISTS deadlines CASCADE;
DROP TABLE IF EXISTS categories CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TYPE IF EXISTS event_invitation_status;
DROP TYPE IF EXISTS share_privacy_level;
DROP TYPE IF EXISTS deadline_priority_level;
DROP TYPE IF EXISTS workload_unit_type;

-- Function to automatically update updated_at timestamp
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'trigger_set_timestamp') THEN
        CREATE FUNCTION trigger_set_timestamp()
        RETURNS TRIGGER AS $func$
        BEGIN
          NEW.updated_at = NOW();
          RETURN NEW;
        END;
        $func$ LANGUAGE plpgsql;
    END IF;
END $$;

-- Users Table
CREATE TABLE users (
    user_id SERIAL PRIMARY KEY,
    display_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    date_of_birth DATE,
    email_verified BOOLEAN DEFAULT FALSE,
    verification_code TEXT,
    verification_code_expires_at TIMESTAMP WITH TIME ZONE,
    reset_code TEXT,
    reset_code_expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    tfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    tfa_secret TEXT NULL
);
DROP TRIGGER IF EXISTS set_timestamp_users ON users;
CREATE TRIGGER set_timestamp_users BEFORE UPDATE ON users FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Categories Table
CREATE TABLE categories (
    category_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    name VARCHAR(255) NOT NULL,
    color VARCHAR(50) NOT NULL,
    is_visible BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    UNIQUE (user_id, name)
);
DROP TRIGGER IF EXISTS set_timestamp_categories ON categories;
CREATE TRIGGER set_timestamp_categories
BEFORE UPDATE ON categories
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();

-- Define ENUM Types
CREATE TYPE deadline_priority_level AS ENUM ('normal', 'important', 'urgent');
CREATE TYPE workload_unit_type AS ENUM ('minutes', 'hours', 'days');
CREATE TYPE event_invitation_status AS ENUM ('pending', 'accepted', 'rejected', 'maybe');
CREATE TYPE share_privacy_level AS ENUM ('full', 'limited');

-- Deadlines Table
CREATE TABLE deadlines (
    deadline_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    category_id INTEGER NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    due_date TIMESTAMP WITH TIME ZONE NOT NULL,
    virtual_due_date TIMESTAMP WITH TIME ZONE, -- For virtual deadlines, this is the date/time of the next occurrence
    priority deadline_priority_level DEFAULT 'normal',
    workload_magnitude INTEGER,
    workload_unit workload_unit_type,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(category_id) ON DELETE SET NULL,
    CONSTRAINT chk_workload CHECK ((workload_magnitude IS NULL AND workload_unit IS NULL) OR (workload_magnitude IS NOT NULL AND workload_unit IS NOT NULL))
);
DROP TRIGGER IF EXISTS set_timestamp_deadlines ON deadlines;
CREATE TRIGGER set_timestamp_deadlines BEFORE UPDATE ON deadlines FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();
DROP TRIGGER IF EXISTS set_virtual_due_date_trigger ON deadlines;
-- Function to automatically set virtual_due_date when NULL
DO $$ BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_proc WHERE proname = 'set_virtual_due_date') THEN
        CREATE FUNCTION set_virtual_due_date()
        RETURNS TRIGGER AS $func$
        BEGIN
          IF NEW.virtual_due_date IS NULL THEN
            NEW.virtual_due_date := NEW.due_date;
          END IF;
          RETURN NEW;
        END;
        $func$ LANGUAGE plpgsql;
    END IF;
END $$;

CREATE TRIGGER set_virtual_due_date_trigger
BEFORE INSERT ON deadlines
FOR EACH ROW
EXECUTE FUNCTION set_virtual_due_date();
-- Events Table
CREATE TABLE events (
    event_id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    category_id INTEGER NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    start_time TIMESTAMP WITH TIME ZONE NOT NULL, -- For recurring, this is the start of the *first* instance
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,   -- For recurring, this defines the duration relative to the start_time
    location VARCHAR(255),
    rrule TEXT,                                   -- Stores the iCalendar RRULE string
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(category_id) ON DELETE SET NULL
);
DROP TRIGGER IF EXISTS set_timestamp_events ON events;
CREATE TRIGGER set_timestamp_events BEFORE UPDATE ON events FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- -- Stores modifications or deletions of specific occurrences within a recurring event series
-- CREATE TABLE event_exceptions (
--     exception_id SERIAL PRIMARY KEY,
--     event_id INTEGER NOT NULL,                          -- Foreign key to the parent recurring event in the 'events' table
--     original_occurrence_time TIMESTAMP WITH TIME ZONE NOT NULL, -- The start time of the occurrence this exception replaces/deletes, as generated by the RRULE

--     is_deleted BOOLEAN NOT NULL DEFAULT FALSE,          -- If true, this specific occurrence is simply cancelled/deleted

--     -- Override fields (NULL means inherit from parent event unless deleted)
--     title VARCHAR(255),
--     description TEXT,
--     start_time TIMESTAMP WITH TIME ZONE,                -- The *new* start time for this modified occurrence
--     end_time TIMESTAMP WITH TIME ZONE,                  -- The *new* end time for this modified occurrence
--     location VARCHAR(255),

--     created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
--     updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

--     FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE CASCADE,
--     UNIQUE (event_id, original_occurrence_time),       -- Ensure only one exception per original occurrence time for a given event

--     -- Check constraint: A modification must provide new timing info
--     CONSTRAINT chk_exception_modification CHECK (is_deleted OR (start_time IS NOT NULL AND end_time IS NOT NULL))
-- );
-- DROP TRIGGER IF EXISTS set_timestamp_event_exceptions ON event_exceptions;
-- CREATE TRIGGER set_timestamp_event_exceptions BEFORE UPDATE ON event_exceptions FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();


-- Event Invitations Table
CREATE TABLE event_invitations (
    invitation_id SERIAL PRIMARY KEY,
    event_id INTEGER NOT NULL,
    owner_user_id INTEGER NOT NULL,
    invited_user_id INTEGER NOT NULL,
    status event_invitation_status DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    FOREIGN KEY (event_id) REFERENCES events(event_id) ON DELETE CASCADE,
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (invited_user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    UNIQUE (event_id, invited_user_id)
);
DROP TRIGGER IF EXISTS set_timestamp_event_invitations ON event_invitations;
CREATE TRIGGER set_timestamp_event_invitations BEFORE UPDATE ON event_invitations FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Calendar Shares Table
CREATE TABLE calendar_shares (
    share_id SERIAL PRIMARY KEY,
    owner_user_id INTEGER NOT NULL,
    shared_with_user_id INTEGER NOT NULL,
    message TEXT,
    privacy_level share_privacy_level NOT NULL DEFAULT 'full',
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    FOREIGN KEY (owner_user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    FOREIGN KEY (shared_with_user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    UNIQUE (owner_user_id, shared_with_user_id)
);
DROP TRIGGER IF EXISTS set_timestamp_calendar_shares ON calendar_shares;
CREATE TRIGGER set_timestamp_calendar_shares BEFORE UPDATE ON calendar_shares FOR EACH ROW EXECUTE FUNCTION trigger_set_timestamp();

-- Calendar Share Categories Table
CREATE TABLE calendar_share_categories (
    share_id INTEGER NOT NULL,
    category_id INTEGER NOT NULL,
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    PRIMARY KEY (share_id, category_id),
    FOREIGN KEY (share_id) REFERENCES calendar_shares(share_id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(category_id) ON DELETE CASCADE
);

-- Table to manage publicly accessible calendar shares
CREATE TABLE open_calendar_shares (
    open_share_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(), -- UUID primary key
    owner_user_id INTEGER NOT NULL,                           -- The user sharing their calendar view
    privacy_level share_privacy_level NOT NULL DEFAULT 'full',
    expires_at TIMESTAMP WITH TIME ZONE,                      -- NULL means never expires
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),        -- For sync/tracking changes
    deleted_at TIMESTAMP WITH TIME ZONE NULL,                 -- For soft delete

    FOREIGN KEY (owner_user_id) REFERENCES users(user_id) ON DELETE CASCADE
    -- No shared_with_user_id or message as it's public
);

-- Trigger for open_calendar_shares table
DROP TRIGGER IF EXISTS set_timestamp_open_calendar_shares ON open_calendar_shares;
CREATE TRIGGER set_timestamp_open_calendar_shares
    BEFORE UPDATE ON open_calendar_shares
    FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();


-- Table to link open shares to specific categories
CREATE TABLE open_calendar_share_categories (
    open_share_id UUID NOT NULL,
    category_id INTEGER NOT NULL,
    PRIMARY KEY (open_share_id, category_id), -- Composite primary key

    FOREIGN KEY (open_share_id) REFERENCES open_calendar_shares(open_share_id) ON DELETE CASCADE,
    FOREIGN KEY (category_id) REFERENCES categories(category_id) ON DELETE CASCADE -- If a category is deleted, remove it from open shares
);


-- Indexes
-- CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE UNIQUE INDEX idx_users_email_active ON users(email) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_categories_user_id ON categories(user_id);

CREATE INDEX IF NOT EXISTS idx_deadlines_user_id ON deadlines(user_id);
CREATE INDEX IF NOT EXISTS idx_deadlines_user_updated ON deadlines(user_id, updated_at);
CREATE INDEX IF NOT EXISTS idx_deadlines_due_date ON deadlines(user_id, due_date);

CREATE INDEX IF NOT EXISTS idx_events_user_id ON events(user_id);
CREATE INDEX IF NOT EXISTS idx_events_user_updated ON events(user_id, updated_at);
CREATE INDEX IF NOT EXISTS idx_events_time_range ON events(user_id, start_time, end_time);
CREATE INDEX IF NOT EXISTS idx_events_rrule ON events(rrule) WHERE rrule IS NOT NULL;

-- Indexes for Event Exceptions
-- CREATE INDEX IF NOT EXISTS idx_event_exceptions_event_id ON event_exceptions(event_id);
-- CREATE INDEX IF NOT EXISTS idx_event_exceptions_original_time ON event_exceptions(event_id, original_occurrence_time); -- For lookup during expansion
-- CREATE INDEX IF NOT EXISTS idx_event_exceptions_updated ON event_exceptions(updated_at); -- For sync

-- Indexes for Event Invitations
CREATE INDEX IF NOT EXISTS idx_event_invitations_event_id ON event_invitations(event_id);
CREATE INDEX IF NOT EXISTS idx_event_invitations_invited_user ON event_invitations(invited_user_id);
CREATE INDEX IF NOT EXISTS idx_event_invitations_status ON event_invitations(invited_user_id, status);
CREATE INDEX IF NOT EXISTS idx_event_invitations_updated ON event_invitations(invited_user_id, updated_at);

-- Indexes for Calendar Shares
CREATE INDEX IF NOT EXISTS idx_calendar_shares_owner ON calendar_shares(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_calendar_shares_shared_with ON calendar_shares(shared_with_user_id);
CREATE INDEX IF NOT EXISTS idx_calendar_share_categories_share_id ON calendar_share_categories(share_id);
CREATE INDEX IF NOT EXISTS idx_calendar_share_categories_category_id ON calendar_share_categories(category_id);

-- Add Indexes for common lookups
CREATE INDEX IF NOT EXISTS idx_open_calendar_shares_owner ON open_calendar_shares(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_open_calendar_shares_deleted_at ON open_calendar_shares(deleted_at);
CREATE INDEX IF NOT EXISTS idx_open_calendar_share_categories_open_share_id ON open_calendar_share_categories(open_share_id);
CREATE INDEX IF NOT EXISTS idx_open_calendar_share_categories_category_id ON open_calendar_share_categories(category_id);

-- Indexes for 2FA
CREATE INDEX IF NOT EXISTS idx_users_tfa_enabled ON users(tfa_enabled);

-- Function to create default categories for a new user ---
DROP FUNCTION IF EXISTS create_default_categories() CASCADE;
CREATE FUNCTION create_default_categories()
RETURNS TRIGGER AS $$
BEGIN
    -- Insert default categories linked to the NEW user_id
    INSERT INTO categories (user_id, name, color, is_visible) VALUES
    (NEW.user_id, 'Classes', '#1abc9c', TRUE),
    (NEW.user_id, 'Assignments', '#3498db', TRUE),
    (NEW.user_id, 'Family/Friends', '#9b59b6', TRUE),
    (NEW.user_id, 'Personal', '#ffff00', TRUE);

    RETURN NEW; -- Important for AFTER INSERT triggers
END;
$$ LANGUAGE plpgsql;

-- Trigger to call the function after a new user is inserted ---
DROP TRIGGER IF EXISTS trigger_create_default_categories ON users;
CREATE TRIGGER trigger_create_default_categories
AFTER INSERT ON users -- Fire after a row is inserted
FOR EACH ROW          -- For each inserted row
EXECUTE FUNCTION create_default_categories(); -- Execute our new function
