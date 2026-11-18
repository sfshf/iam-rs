# Domain Server

Domain Server is a gRPC-based microservice responsible for managing domains and their associated roles, resources, and permission definitions.

It provides centralized management of domain entities and their relationships, serving as the foundational domain management layer for the distributed system.

The core service interfaces include:

- **Domain Management**: Create, retrieve, update, delete, and list domains.
- **Role Management**: Create, retrieve, update, delete, and list role definitions within a domain.
- **Resource Management**: Register, retrieve, update, delete, and list resource definitions within a domain.
- **Permission Definition Management**: Define and manage available permissions and their associated resources and actions.

Authorization policies, role assignments, permission assignments, and access control evaluation are handled independently by the Authz Server.

---

<p align="center">
← <a href="identity.md">Identity Server</a> &nbsp; | &nbsp; <a href="authz.md">Authz Server</a> →
</p>
