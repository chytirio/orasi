# Orasi UI

A modern web UI for the Orasi data platform built with Leptos.

## Features

- **Modern UI**: Built with Leptos and Tailwind CSS
- **Responsive Design**: Mobile-first responsive layout
- **Routing**: Client-side routing with Leptos Router
- **Type Safety**: Full Rust type safety throughout the application

## Prerequisites

- Rust (latest stable version)
- `wasm-pack` for building WebAssembly
- A web server to serve the built files

## Installation

1. Install `wasm-pack`:
```bash
cargo install wasm-pack
```

2. Build the project:
```bash
wasm-pack build --target web
```

3. Serve the files:
```bash
# Using Python 3
python3 -m http.server 8080

# Or using Node.js
npx serve .

# Or using any other static file server
```

4. Open your browser and navigate to `http://localhost:8080`

## Development

For development with hot reloading, you can use tools like:

- `trunk` (recommended for Leptos development)
- `live-server`
- Any development server with file watching

### Using Trunk

1. Install trunk:
```bash
cargo install trunk
```

2. Run the development server:
```bash
trunk serve
```

## Project Structure

```
ui/
├── src/
│   ├── lib.rs          # Main entry point
│   ├── app.rs          # Main App component
│   ├── pages/          # Page components
│   │   ├── mod.rs
│   │   ├── home.rs     # Home page
│   │   ├── dashboard.rs # Dashboard page
│   │   └── not_found.rs # 404 page
│   └── components/     # Reusable components
│       └── mod.rs
├── Cargo.toml          # Rust dependencies
├── index.html          # HTML template
└── README.md           # This file
```

## Building for Production

```bash
wasm-pack build --target web --release
```

The built files will be in the `pkg/` directory.

## Integration with Orasi Backend

This UI is designed to work with the Orasi data platform backend. You can:

1. Configure API endpoints in the components
2. Add authentication and authorization
3. Connect to real-time data streams
4. Integrate with the query engine

## Contributing

1. Follow Rust coding standards
2. Use Leptos best practices
3. Ensure responsive design
4. Add tests for new components

## License

This project is part of the Orasi data platform.
