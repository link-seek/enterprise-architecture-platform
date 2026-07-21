# Enterprise Architecture Platform (EAP)

Enterprise Architecture Platform — Rust + React Full-Stack Demo (EAP) is a full-stack enterprise application built with a Domain-Driven Design (DDD) architecture.

The platform consists of two main components:

- **Backend** (`backend/`): A Rust-based service built with `axum`, `sea-orm`, and `SQLite`, exposing a GraphQL API. It follows a modular DDD layout with crates for shared common code, user management, and business architecture.
- **Frontend** (`frontend/`): A React single-page application using `shadcn/ui`, `Apollo Client` (GraphQL), and `Tailwind CSS`, built with Vite.

## Project Structure

```
.
├── backend/      # Rust backend (axum + sea-orm + SQLite + GraphQL)
├── frontend/     # React frontend (Vite + shadcn/ui + Apollo Client)
├── scripts/      # Deployment and automation scripts
└── docker-compose.ci.yml
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
