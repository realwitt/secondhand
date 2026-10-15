**Secondhand 0.1.0: the first release.** Drop old FreeHand 8, 9, 10 and MX drawings (or whole folders of them) on Secondhand, and it saves an `.ai` file next to each one that current Illustrator opens directly. It can also save PDF or SVG.

- Pages become artboards, layers keep their names and order (hidden layers stay hidden), and paths, groups, clipping, colours and gradients come across as editable artwork. See [what comes across](https://github.com/realwitt/secondhand#what-comes-across-and-what-doesnt).
- Batches of any size. Folders are searched all the way down, and one bad file never stops the rest. At the end you get a list of what was converted, skipped or failed, and why.
- Your originals are never changed, and nothing is overwritten: if `Name.ai` already exists you get `Name (1).ai`.
- Everything runs on your computer, with no internet connection.
- FreeHand 7 and older are recognised and politely refused, with the version named.

### Which file do I download?

| Computer | File |
|---|---|
| **Mac** (Apple silicon and Intel) | `Secondhand_0.1.0_universal.dmg`. If the .dmg gives you trouble, use `Secondhand_0.1.0_universal.app.zip` instead |
| **Windows 10/11** | `Secondhand_0.1.0_x64-setup.exe` (installer). Also available: `Secondhand_0.1.0_x64_en-US.msi`, or `Secondhand_0.1.0_x64_portable.exe`, which runs without installing |
| Linux (command line only, for testing) | `secondhand-cli-linux-x64` |

### The first time you open it

Secondhand isn't signed with a paid Apple or Microsoft certificate, so your computer will ask once whether you trust it.

**Mac**
1. Drag **Secondhand** from the .dmg into **Applications**, then double-click it.
2. macOS says *"Secondhand" Not Opened*. Click **Done**.
3. Open  ▸ **System Settings ▸ Privacy & Security**, scroll down to **Security**, and next to *"Secondhand" was blocked to protect your Mac* click **Open Anyway**.
4. Enter your password (or use Touch ID) and click **Open Anyway** again. Done. It opens normally from now on.

**Windows**
1. Run the installer (or the portable .exe).
2. If a blue *Windows protected your PC* box appears, click **More info**, then **Run anyway**.

Secondhand is open source (MIT OR Apache-2.0), built on the [VectorCraft](https://github.com/storytold/vectorcraft) engine, and isn't affiliated with Adobe. FreeHand and Illustrator are Adobe trademarks, named only to describe the files Secondhand reads and writes.
