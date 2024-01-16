# Tauri macOS Spotlight Example

This is an example project that shows how to create a macOS Spotlight app using Tauri.

> Check out the [file-upload](https://github.com/ahkohd/tauri-macos-spotlight-example/tree/file-upload) branch for an example of the spotlight app, now featuring file upload functionality. See it in action [here](https://github.com/ahkohd/tauri-macos-spotlight-example/issues/12#issuecomment-1826410794).

This template is based on Tauri + React + Typescript template. It should help get you started developing with Tauri, React and Typescript in Vite.

## Prerequisites

- _[<ins>Node.js<ins>](https://nodejs.org)_
- _[<ins>Tauri CLI<ins>](https://tauri.studio/docs/getting-started/installation)_

## Getting Started

1. Clone this repository:

```
git clone https://github.com/ahkohd/tauri-macos-spotlight-example.git
```

2. Navigate to the project directory:

```
cd tauri-macos-spotlight-example
```

3. Run the demo

```
pnpm install
pnpm tauri dev
```

4. Press <kbd>Cmd</kbd><kbd>k</kbd> to toggle the spotlight window

![Demo](./demo.gif)

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

# Related

The following are related to this project:

- [tauri-nspanel](https://github.com/ahkohd/tauri-nspanel/tree/main/examples/vanilla): A Tauri plugin that enables the transformation of a standard application window into a panel, providing panel-specific functionalities and methods. It is designed for versatile application across various projects.
- [tauri-plugin-spotlight](https://github.com/zzzze/tauri-plugin-spotlight): Also a Tauri plugin that helps you to emulate a spotlight window behavior. Unlike the `tauri-nspanel` or this example project, it does not utilize a panel. As a result, its ability to draw over fullscreen applications on newer macOS versions may be limited.

# License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE.md) file for details.
