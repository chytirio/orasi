# Orasi Documentation

This directory contains the Orasi project documentation built with [Zola](https://www.getzola.org/) and the [Adidoks](https://github.com/aaranxu/adidoks) theme.

## Prerequisites

Before building the documentation, you need to install Zola:

### macOS
```bash
brew install zola
```

### Linux
```bash
# Download and install from https://github.com/getzola/zola/releases
# Or use your package manager
```

### Windows
```bash
# Download from https://github.com/getzola/zola/releases
# Or use Chocolatey: choco install zola
```

## Building the Documentation

### Development Server

To start a development server with live reload:

```bash
# From the book directory
zola serve

# Or from the project root
cd book && zola serve
```

The documentation will be available at `http://127.0.0.1:1111`

### Build for Production

To build the static site for production:

```bash
# From the book directory
zola build

# Or from the project root
cd book && zola build
```

The built site will be in the `public/` directory.

### Build with Base URL

For deployment with a specific base URL:

```bash
zola build --base-url https://orasi.dev
```

## Configuration

The documentation is configured in `config.toml`. Key settings include:

- **Theme**: Uses the Adidoks theme for a clean, modern look
- **Navigation**: Configured in the `[extra.nav]` section
- **Social Links**: GitHub, Twitter, and Discord links
- **Search**: Full-text search is enabled
- **Taxonomies**: Tags and categories for organizing content

## Content Structure

```
content/
├── _index.md              # Home page
├── getting-started/       # Getting started guide
│   └── _index.md
├── components/            # Component documentation
│   └── _index.md
├── api/                   # API reference
│   └── _index.md
└── examples/              # Examples and tutorials
    └── _index.md
```

## Adding Content

### Creating a New Page

1. Create a new `.md` file in the appropriate directory
2. Add front matter at the top:

```markdown
+++
title = "Page Title"
description = "Page description"
weight = 1
+++

# Page Content

Your content here...
```

### Front Matter Options

- `title`: Page title
- `description`: Page description for SEO
- `weight`: Order in navigation (lower numbers appear first)
- `template`: Custom template (optional)
- `page_template`: Custom page template (optional)

### Adding to Navigation

Edit `config.toml` and add entries to the `[[extra.nav.links]]` section:

```toml
[[extra.nav.links]]
text = "Your Page"
url = "/your-page/"
```

## Theme Customization

The Adidoks theme can be customized by:

1. **Colors**: Modify CSS variables in the theme
2. **Layout**: Override theme templates in `templates/`
3. **Styling**: Add custom CSS in `static/`

## Deployment

### GitHub Pages

1. Build the site: `zola build`
2. Push the `public/` directory to the `gh-pages` branch
3. Configure GitHub Pages to serve from the `gh-pages` branch

### Netlify

1. Connect your repository to Netlify
2. Set build command: `cd book && zola build`
3. Set publish directory: `book/public`

### Vercel

1. Connect your repository to Vercel
2. Set build command: `cd book && zola build`
3. Set output directory: `book/public`

## Contributing

When contributing to the documentation:

1. Follow the existing content structure
2. Use clear, concise language
3. Include code examples where appropriate
4. Test your changes locally before submitting
5. Update the navigation if adding new sections

## Troubleshooting

### Common Issues

**Theme not found:**
- Ensure the adidoks theme is cloned in `themes/adidoks/`
- Check that the theme path in `config.toml` is correct

**Build errors:**
- Check for syntax errors in markdown files
- Verify front matter is properly formatted
- Ensure all referenced files exist

**Missing assets:**
- Check that static files are in the `static/` directory
- Verify file paths in templates and markdown

### Getting Help

- [Zola Documentation](https://www.getzola.org/documentation/)
- [Adidoks Theme Documentation](https://github.com/aaranxu/adidoks)
- [GitHub Issues](https://github.com/your-org/orasi/issues)
