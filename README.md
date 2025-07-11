# avim - A Vim-Inspired Audio Editor

`avim` is a terminal-based audio editor designed for speed and efficiency, especially for spoken-word content like podcasts, interviews, and audiobooks. It transcribes your audio into intelligently segmented clips and provides a modal, keyboard-centric interface inspired by the Vim text editor.

Instead of clicking and dragging on a timeline, you edit audio by manipulating the text of its transcript.

## Features

* **Intelligent Transcription:** Uses the Gemini API to transcribe audio and segment it into logical clips based on speaker changes and pauses.
* **Vim-like Modal Editing:** Navigate and edit audio clips using familiar Vim keybindings (`j`, `k`, `dd`, `yy`, `p`).
* **Transcript Adjustment:** Powerful modes to correct transcription errors by moving text between clips or adjusting timestamps.
* **Intelligent Autofix:** Learns from your manual corrections to suggest and apply fixes to the rest of the file.
* **Project Files:** Save your editing session, including all clips and comments, to a `.avim` file to resume work later.
* **Terminal-Based:** Runs entirely in your terminal, making it lightweight and accessible via SSH.

## Getting Started

### Installation & Running

1.  **Install Dependencies:** Follow the instructions in the [Technical Specs](immersive://avim_tech_specs) document to install Rust, SoX, and other required libraries.
2.  **Set API Key:** Set your `GEMINI_API_KEY` environment variable.
3.  **Run the application:**
    * To start a new project: `avim your_audio_file.wav`
    * To resume an existing project: `avim your_project_file.avim`

### Startup Flags

You can modify the application's behavior at launch with the following flags:

| Flag         | Description                                                                    | Example                             |
| :----------- | :----------------------------------------------------------------------------- | :---------------------------------- |
| `--no-cache` | Ignores any existing cache file and forces a new transcription from the Gemini API. | `avim --no-cache my_audio.wav`  |
| `--debug`    | Displays an interactive debug panel showing internal state and logs for testing. | `avim --debug my_audio.wav`     |

## The `avim` Workflow

The recommended workflow is designed to be fast and efficient:

1.  **Initial Transcription:** `avim` creates an initial transcript. It automatically validates this against the true audio length and sanitizes it to remove "phantom" clips.
2.  **Manual Correction (The "Learning" Phase):** Use the `m` (adjust) command to fix the first few clips where the text doesn't perfectly match the audio segment. After 2-3 consistent adjustments, `avim` will learn your correction pattern.
3.  **Intelligent Autofix:** Run the `:autofix` command. `avim` will use the pattern it learned to automatically correct the rest of the file. You can repeat steps 2 and 3 as needed to further refine the transcript.
4.  **Final Edits:** Use the standard Vim motions (`dd`, `p`, etc.) and timestamp nudging (`[`, `]`, `{`, `}`) to make your final creative edits.
5.  **Save and Export:** Save your work to a `.avim` project file with `:w` and export the final audio with `:export`.

## Command Reference

### Normal Mode

| Key(s)     | Action              | Description                                                  |
| :--------- | :------------------ | :----------------------------------------------------------- |
| `j` / `k`  | Navigate Clips      | Move the selection down or up.                               |
| `dd`       | Delete Clip         | Deletes the currently selected clip.                         |
| `yy`       | Yank Clip           | Copies (yanks) the current clip to the register.             |
| `p`        | Paste Clip          | Pastes the yanked clip after the current selection.          |
| `u`        | Undo                | Reverts the last action.                                     |
| `Ctrl`+`r` | Redo                | Re-applies the last undone action.                           |
| `spacebar` | Play/Stop Clip      | Toggles playback for the currently selected clip.            |
| `Shift`+`P`| Play/Stop All       | Toggles playback for all clips from the current one to the end. |
| `[` / `]`  | Adjust Start Time   | Nudges the start time of the clip backward/forward by 50ms.  |
| `{` / `}`  | Adjust End Time     | Nudges the end time of the clip backward/forward by 50ms.  |
| `m`        | Enter Adjust Mode   | Enters transcript adjustment mode.                           |
| `i`        | Enter Insert Mode   | Enters Insert Mode to add a comment.                         |
| `:`        | Enter Command Mode  | Switches to Command Mode.                                    |

### Adjust Mode (`m`)

| Key(s)     | Action               | Description                                                  |
| :--------- | :------------------- | :----------------------------------------------------------- |
| `w` / `b`  | Select Word          | Moves the split point forward/backward one word in the next clip. |
| `Enter`    | Confirm Adjustment   | Moves the selected words to the current clip's transcript.   |
| `Esc`      | Cancel               | Exits Adjust Mode without making changes.                    |

### Command Mode (`:`)

| Command                        | Description                                                  |
| :----------------------------- | :----------------------------------------------------------- |
| `:w [filename.avim]`           | Saves the current state to an `.avim` project file.          |
| `:export {format} {filename}`  | Exports the final edited audio.                              |
| `:q` / `:q!`                   | Quits the application.                                       |
| `:help`                        | Displays a summary of all available commands.                |
| `:lasterror`                   | Copies the last recorded error message to the system clipboard. |
| `:autofix`                     | Applies the learned text adjustments to the rest of the file. |
