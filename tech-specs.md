avim - Technical Specifications
This document outlines the technical architecture, core logic, and development history of the avim application.

1. Code Modularization Strategy
The application is structured into several modules for maintainability and clarity.

~/avim/
└── src/
    ├── main.rs         # Entry point, main loop, and task spawning
    ├── app.rs          # Core application state (App struct) and logic
    ├── ui.rs           # All UI rendering logic (ui function)
    ├── gcp.rs          # Gemini API interaction logic
    ├── sox.rs          # SoX command execution (play and export)
    ├── cache.rs        # Logic for reading from and writing to the cache
    ├── vim.rs          # Core editor motions (dd, yy, p, j, k, etc.)
    └── autofix.rs      # "Funky math" logic for intelligent transcript correction

2. Core Logic and Workflow
2.1. Initial Loading and Caching
To improve performance and reduce cost, avim implements a transcription caching system. When an audio file is loaded:

The app first checks for a local cache file corresponding to the audio file.

Cache Hit: If a valid cache exists, the transcription is loaded instantly, bypassing the API.

Cache Miss: If no cache exists, the app proceeds to the transcription stage.

2.2. Transcription and Sanitization
Chunking: The audio file is split into 5-minute chunks to avoid API limits with large files.

Transcription: Each chunk is sent to the Gemini API for transcription.

Ground Truth Validation: After all chunks are transcribed, the application gets the true audio duration using soxi -D.

Sanitization: The application then filters the list of clips from the API. Any clip starting after the true audio duration is discarded, and the final clip's end time is trimmed to match the true duration. This prevents "phantom" clips from appearing.

2.3. The "Funky Math" Autofix Model
The :autofix command is designed to learn from the user's manual corrections and apply them to the rest of the file. This is a recursive, continuous learning process.

Data Collection: When the user manually adjusts a clip with the m command, the app records the number of words moved.

Continuous Learning: After every manual adjustment, the app re-calculates the mean and standard deviation of all adjustments made so far.

Confidence Threshold: If the standard deviation of the word counts is low (e.g., < 1.0), the application gains confidence that it has learned a consistent pattern.

Intelligent Autofix: When :autofix is run, the app uses the calculated average number of words to move and applies this correction to the rest of the clips in the file. The user can run this command multiple times, and the model will use the full history of adjustments for its calculations.

4. Command Reference
4.1. Launch Flags
Flag

Description

Example

--no-cache

Ignores any existing cache file and forces a new transcription from the Gemini API.

avim --no-cache my_audio.wav

--debug

Displays an interactive debug panel showing internal state and logs.

avim --debug my_audio.wav

4.2. Normal Mode
Key(s)

Action

Description

j / k

Navigate Clips

Move the selection down or up.

dd

Delete Clip

Deletes the currently selected clip.

yy

Yank Clip

Copies (yanks) the current clip to the register.

p

Paste Clip

Pastes the yanked clip after the current selection.

u

Undo

Reverts the last action that changed the clips.

Ctrl+r

Redo

Re-applies the last undone action.

spacebar

Play/Stop Clip

Toggles playback for the currently selected clip.

Shift+P

Play/Stop All

Toggles playback for all clips from the current one to the end.

[ / ]

Adjust Start Time

Nudges the start time of the clip backward/forward by 50ms.

{ / }

Adjust End Time

Nudges the end time of the clip backward/forward by 50ms.

m

Enter Adjust Mode

Enters transcript adjustment mode for the current clip.

i

Enter Insert Mode

Enters Insert Mode to add a comment to the current clip.

:

Enter Command Mode

Switches to Command Mode.

4.3. Adjust Mode
Key(s)

Action

Description

w

Select Next Word

Moves the split point forward one word in the next clip.

b

Select Previous Word

Moves the split point backward one word in the next clip.

Enter

Confirm Adjustment

Moves the selected words to the current clip's transcript.

Esc

Cancel

Exits Adjust Mode without making changes.

4.4. Insert Mode
Key(s)

Action

Description

(any text)

Type Comment

Text is added as a comment to the clip, prefixed with //.

Esc

Return to Normal Mode

Exits Insert Mode.

4.5. Command Mode
Command

Description

:w [filename.avim]

Saves the current state to an .avim project file.

:export {format} {filename}

Exports the final edited audio to the specified format.

:q / :q!

Quits the application (with or without saving).

:help

Displays a summary of all available commands.

:lasterror

Copies the last recorded error message to the system clipboard.

:autofix

Applies the learned text adjustments to the rest of the file.

5. Build-Time Issues and "Gotchas"
Issue

Error Message / Symptom

Cause & Fix

OpenSSL Linking

failed to run custom build command for openssl-sys

Cause: reqwest requires native OpenSSL development libraries. Fix: sudo apt install libssl-dev pkg-config.

Thread Safety

future cannot be sent between threads safely

Cause: Default Box<dyn Error> is not thread-safe. Fix: Changed signatures to Box<dyn Error + Send + Sync>.

Missing Dependency

use of unresolved module or unlinked crate

Cause: A feature was implemented without adding the crate to Cargo.toml. Fix: Added textwrap, arboard, etc.

Borrow Checker

cannot borrow as mutable because it is also borrowed as immutable

Cause: Attempting simultaneous mutable and immutable borrows. Fix: Refactored logic to release immutable borrow first.

Key Sequence Logic

dd and yy commands were unreliable.

Cause: A Tick event was resetting the last_key state too quickly. Fix: Removed the state reset from the Tick handler.

Screen Corruption

sox WARN messages appeared in the UI.

Cause: sox was printing warnings to stderr. Fix: Redirected stdout and stderr for background sox commands to Stdio::null().

Unclean Exit

Cursor not returning and audio playing after quit.

Cause: The app was not killing the child sox process on exit. Fix: Added logic to kill the playback_pid before restoring the terminal.

Invisible Cursor

Cursor was not visible in Adjust (m) mode.

Cause: Incorrect UI logic for calculating cursor position. Fix: Updated UI rendering to explicitly calculate and set the cursor position.

Panic on Adjust

App panicked when confirming an adjustment.

Cause: Unsafe string splitting logic. Fix: Replaced with a robust join(" ") method on the word vector.

API Truncation

JSON parsing failed on large files.

Cause: Audio file was too large for a single API request. Fix: Implemented audio chunking.

Phantom Clips

Transcription was longer than the audio.

Cause: The API "hallucinated" timestamps. Fix: Implemented soxi -D validation and sanitization.


