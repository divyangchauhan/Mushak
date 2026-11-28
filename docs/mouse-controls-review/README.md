# Mouse controls, 30 design rounds

Three of four groups finished at 9. The wheel editor finished at 8. The remaining concern is the small gap between the last scrolling option and the discard prompt. At 720 × 360, the two wheel columns also need more separation.

[Before-and-after gallery](gallery.html) · [All 372 reviewer ballots, with the eight scores and visual observations](scores.csv) · [Medians for each round](medians.json) · [Native shortcut sequence](native/)

| Group | Baseline | Round 30 |
| --- | ---: | ---: |
| Overview, diagram, profile chooser and compact lists | 8 | 9 |
| Scroll wheel and middle click | 8 | 8 |
| Button actions, search and keyboard shortcut | 9 | 9 |
| Wheel mode button and pointer speeds | 8 | 9 |

## What moved during editing

At 968 × 668, six switches between Scrolling and Middle click produced the same wheel marker bounds, x 298.48, y 326.09, width and height 28.32. The wheel panel stayed at x 575.32, y 144.00 and Save stayed at x 805, y 565. On Back, both the action and shortcut tab rows stayed at y 208 and both Save controls stayed at y 565. At 720 × 360 both editors placed Save and Cancel at y 303 with a height of 32, leaving 25 logical pixels to the bottom of the window. Both scrolling switches and the dead-zone slider fit above that footer. [Dark measurements](round-30/Dark/bounds.csv) · [Light measurements](round-30/Light/bounds.csv)

The release executable opened Settings in Windows processes 58996 and 49216. The script selected SmartShift, saved Ctrl+C for middle click, and cycled the Back editor between its action and shortcut tabs three times. It recorded M, resized with the editor open, canceled, recorded K, saved, and restarted Settings. The saved mapping and wheel mode reopened. Comparing the complete parsed configuration found only the intended edits. [Recorded M](native/07-recorded-M.webp) · [After resize](native/08-resized.webp) · [Saving K](native/08b-save-K.webp) · [Reopened Back](native/10-reopened-back.webp) · [Receipt](native/receipt.json)

## Changes and evidence

Each row links to an actual frame from that round. Select the corresponding frame and theme in the gallery to compare it with an earlier round. The extra frames for discard confirmation, modifier selection, the compact profile menu, a longer DPI list, and a selected search result first appear in rounds 25, 26, 27, 28 and 30 respectively.

| Round | Interaction | Change | Evidence |
| ---: | --- | --- | --- |
| 1 | Disconnect at compact width | Hid the stale battery reading and reduced the compact rail padding. | [A4](round-01/Dark/A4.webp) · [98103722](round-01/capture.json) |
| 2 | Resize with the wheel editor open | Put wheel mode and scroll options in two columns so the footer and both switches fit. | [B4](round-02/Dark/B4.webp) · [0a408597](round-02/capture.json) |
| 3 | Compare the connected and offline wheel rows | Kept the wheel name and shortened the default middle-click description. The compact rail uses Controls. | [A4](round-03/Dark/A4.webp) · [6b04a4f8](round-03/capture.json) |
| 4 | Open Set speeds | Removed action search from the numeric editor and kept Back in a consistent place. | [D2](round-04/Dark/D2.webp) · [4996b428](round-04/capture.json) |
| 5 | Match a marker to its action card | Moved the card column closer to the mouse and used the extra width for assignment text. | [A1](round-05/Dark/A1.webp) · [73f64a95](round-05/capture.json) |
| 6 | Open Middle click and browse Common | Removed redundant headings in the wheel editor and fit all six common actions. | [B3](round-06/Dark/B3.webp) · [f0045fd0](round-06/capture.json) |
| 7 | Edit two pointer speeds in a short window | Gave each numeric field a full-width slider underneath. | [D2](round-07/Dark/D2.webp) · [9f84d6a5](round-07/capture.json) |
| 8 | Compare Copy in the catalog and search | Used one chord formatter for both rows. | [C2](round-08/Dark/C2.webp) · [4be45541](round-08/capture.json) |
| 9 | Read the category rail | Widened the rail to fit Tabs & windows on one line. | [B3](round-09/Dark/B3.webp) · [603831b7](round-09/capture.json) |
| 10 | Read the offline message beside the diagram toggle | Shortened the reconnection message to fit. | [A4](round-10/Dark/A4.webp) · [49eec3e5](round-10/capture.json) |
| 11 | Compare the slider with values reported by the mouse | Listed supported DPI steps. Later rounds replaced short lists with direct buttons. | [D2](round-11/Dark/D2.webp) · [a905c045](round-11/capture.json) |
| 12 | Change SmartShift to Ratchet without saving | Added an unsaved indicator next to Save, including when switching tabs. | [B2](round-12/Dark/B2.webp) · [d5655c86](round-12/capture.json) |
| 13 | Record K and inspect the pending shortcut | Labeled the pending shortcut and its Record again command. | [C3](round-13/Dark/C3.webp) · [944759c8](round-13/capture.json) |
| 14 | Open a mapping from a compact row | Replaced separate Edit badges with a single row affordance that matches the diagram list. | [A3](round-14/Dark/A3.webp) · [06691e5c](round-14/capture.json) |
| 15 | Click a scrolling switch label, then its switch | Made both targets clickable and added hover feedback. The regression check verifies exactly one change per click. | [B1](round-15/Dark/B1.webp) · [e806d648](round-15/capture.json) |
| 16 | Read sensitivity immediately below the mode buttons | Separated the controls and explained how a lower threshold affects free-spin. | [B4](round-16/Dark/B4.webp) · [1e9e7d7e](round-16/capture.json) |
| 17 | Match DPI numbers to the two sliders | Placed supported step labels under each slider. Direct selection superseded these labels in round 28. | [D2](round-17/Dark/D2.webp) · [d173d0e1](round-17/capture.json) |
| 18 | Click a numbered marker on the diagram | Enlarged markers around their existing centers. | [A1](round-18/Dark/A1.webp) · [ba27717c](round-18/capture.json) |
| 19 | Read the last common action over the footer | Used the category name as its heading and reduced row height. | [C1](round-19/Dark/C1.webp) · [1c1268a6](round-19/capture.json) |
| 20 | Find the wheel's scrolling controls from the list | Named scrolling alongside the middle-click mapping when the mouse reports scroll support. | [A1](round-20/Dark/A1.webp) · [764efbcc](round-20/capture.json) |
| 21 | Move from scrolling to the two ways of assigning middle click | Gave nested assignment methods a different selection treatment and named the profile scope. | [B3](round-21/Dark/B3.webp) · [f6ef9da6](round-21/capture.json) |
| 22 | Search for Copy | Added the number of matches in each result category. | [C2](round-22/Dark/C2.webp) · [37423de6](round-22/capture.json) |
| 23 | Compare the title and profile picker at 720 pixels | Right-aligned and shortened the compact picker. | [A3](round-23/Dark/A3.webp) · [6d2f6360](round-23/capture.json) |
| 24 | Preview the result of a speed change | Grouped current and next-press speeds. Validation shares the same fixed feedback row. | [D2](round-24/Dark/D2.webp) · [05af475b](round-24/capture.json) |
| 25 | Click another marker with an unsaved wheel edit | Grouped Keep editing and Discard edits. Reviewers asked for more context, addressed in round 29. | [B5](round-25/Dark/B5.webp) · [a470ef26](round-25/capture.json) |
| 26 | Add and remove Ctrl from a recorded K | Increased selected modifier contrast and exposed its selected state to accessibility clients. | [C5](round-26/Dark/C5.webp) · [775a09fe](round-26/capture.json) |
| 27 | Open the compact profile chooser | Anchored its menu directly under the picker and checked the gap. Round 30 moved the diagram command clear of it. | [A5](round-27/Dark/A5.webp) · [520e486a](round-27/capture.json) |
| 28 | Pick among five reported DPI values | Replaced the short-list slider with labeled, clickable values. A 32-value fixture still uses the slider. | [D2](round-28/Dark/D2.webp) · [082f9f27](round-28/capture.json) |
| 29 | Click a different marker with a pending edit, then choose each response | Named what Discard does, increased the contrast of both options, and checked that discarding switches controls without writing. Also gave the offline note a status icon. | [B5](round-29/Dark/B5.webp) · [bdb84983](round-29/capture.json) |
| 30 | Select a search result and open the compact profile menu | Made Select and the pending state easier to see, moved Show diagram clear of the profile menu, and displayed the endpoints of long DPI ranges. | [C6](round-30/Dark/C6.webp) · [79226ba3](round-30/capture.json) |

## Review method and coverage

GPT-6.2 Astra implemented the changes and had no vote. GPT-6.2 Sol, GPT-6.1 Terra and GPT-6 Astra each received the actual image sheets in a new context for each of the 31 versions, including baseline. Neutral packet names hid the round. Reviewers saw the fixed anchors and rubric without the target, previous scores, other ballots or source. Each scored all four groups, provided a visible label and described a specific detail. This produced 93 independent reviews of 906 frames across the series and 372 group ballots. The final panel inspected 38 frames per reviewer. [Review prompt](ballot-instructions.txt)

Each group’s Overall is the median of three separate judgments. Reviewers scored typography, hierarchy, spacing and density, color and contrast, iconography, consistency and polish, clarity, and Overall from 0 to 10. Overall has its own score. An 8 means a good screen with a material issue, a 9 means excellent, and a 10 means exceptional. The CSV retains fractional scores. Differences between rounds include differences between panels.

Dark and Light both used Marigold at 200% Windows display scale. The single active display reported 3000 × 1876 and 192 DPI. Captures use 968 × 668, 720 × 540, and 720 × 360 logical clients. Each version has its capture metadata and verification receipt. Actual GPU frames were reduced to logical resolution and saved as lossless WebP. Reviewers saw JPEG contact sheets made from those frames. The release-process screenshots use a separate isolated configuration. [Display enumeration](native/displays.json)

The runner loads a recorded Bluetooth MX Master 2S descriptor, simulates disconnection, and supplies a SmartShift maximum of 50, an eight-step wheel multiplier and five DPI choices. The later long-list fixture supplies 32 DPI choices from 100 to 3200. The application gates the controls on the reported capabilities. Physical remapping, wheel feel, firmware application, other physical mice and another Windows display scale remain unverified in this run. Only the four Mouse controls groups received fresh council scores.

## Source and verification

Base commit `30a68755c9e84106f3ec12b2578eaff1b8a54817` contains the wheel editor the owner was testing. Its parent, `c4f6c025669e72005add83426db4b8b787c05f41`, has the same tree as the merged release branch at `cf239839104303f13dce324edd36a04ac890d837`. The release tag v0.0.11 points at `096d2c1693a3d390a7059dbd558aaa5274a05c88`, an ancestor of the base. Fetched origin/main was `a1d161dc58795119f099aac7128fbc60084b788c`; it had one unique commit and the chosen base had 45. The separate worktree is on `design/mouse-controls-30`. Baseline capture commit `6cd6850bd0ead8d81c482f5fb5137b0af8021a91` added the debug capture driver to the base UI.

Final visual source `79226ba3ab09a84f25c25ed808082fb51909df1f`. The subsequent test-only commit `f3a8d383fc203e9e33a2f4c451af7e8783372a39` updated the full-gallery bounds assertion to check the speed form instead of a hidden search field. Each design round built with Cargo’s locked dependencies, then exercised both themes in the Windows renderer. The release build passed, 62 Rust tests passed, and the general Settings smoke run passed. The full Settings gallery also passed after the bounds assertion was corrected. The final release executable had SHA-256 `30A7FDC4F96579D04B18DDCB8C10974837EB536F127A3C31FD61173B85331FA4`. [Build and test results](validation.json) · [Native verification script](verify-native.py)

To repeat the focused interaction checks, run `cargo run --locked -- --ui-smoke --controls-review` from the worktree. To repeat the separate process check, build with `cargo build --release --locked --target-dir target/native` and run `python docs/mouse-controls-review/verify-native.py`. It uses temporary sample data. The worktree release binary is at `target/native/release/mushak.exe`.

## Remaining observations

The wheel discard prompt can introduce a short scrollbar and sits close to the final helper line. The compact profile menu overlaps the upper-right portion of the first mapping row. Its label and diagram toggle remain visible. A single search result leaves room above the fixed footer. The numeric DPI field and short-list presets both edit the same speed, which one reviewer found ambiguous. The panel did not report an inaccessible Save or Cancel button. [Final ballots](scores.csv)
