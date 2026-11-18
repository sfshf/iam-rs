# Authz Server

Authz Server is a gRPC-based microservice responsible for managing authorization policies, permission grants, and access control evaluations across the distributed system.

It provides centralized authorization management and policy enforcement capabilities, allowing other services to manage access control configurations, grant or revoke permissions, and evaluate authorization requests through a unified interface.

The core service interfaces include:

- **Authorization Policy Management**: Create, retrieve, update, delete, and list authorization policies.
- **Role Assignment Management**: Assign roles to users or other subjects within a specific domain, as well as revoke and query role assignments.
- **Permission Grant Management**: Grant or revoke permissions to users, roles, or other subjects for specific resources and actions.
- **Access Control Evaluation**: Evaluate authorization requests and determine whether a subject is allowed to perform a specific action on a resource within a given domain.
- **Role and Permission Query**: Retrieve the roles assigned to a subject and the permissions associated with a subject or role.
- **Policy Synchronization**: Synchronize role, resource, and permission definitions from the Domain Service to ensure authorization policies remain consistent with domain metadata.

Authz Service uses [Casbin](https://github.com/apache/casbin-rs) as its underlying authorization engine to evaluate access control policies. It encapsulates the authorization logic and policy management behind a set of gRPC APIs, preventing other services from depending directly on Casbin.

---

<p align="center">
← <a href="domain.md">Domain Server</a> &nbsp; | &nbsp; <a href="audit.md">Audit Server</a> →
</p>
