#!/usr/bin/env python3

import requests
import argparse
import json


def check_response(response):
    """Check API response and return result"""
    if response.status_code != 200:
        print(f"Error: HTTP {response.status_code} - {response.text}")
        exit(1)

    try:
        response_json = response.json()
        if "error" in response_json:
            print(f"Error: {response_json['error']}")
            exit(1)
        return response_json
    except ValueError:
        return response.text or "Success"


def view_admin_roles(url, token, name=None, role_type=None):
    """View all admin roles"""
    headers = {"Authorization": f"Bearer {token}"}
    pageSize = 30
    params = {}

    if name:
        params["name"] = name
    if role_type:
        params["type"] = role_type

    filtered_params = {
        k: "%" + v + "%" if (v != "-" and "%" not in v and k != "type") else v
        for k, v in params.items()
        if v is not None
    }
    filtered_params["pageSize"] = pageSize

    roles = []
    current = 0

    while True:
        current += 1
        filtered_params["current"] = current
        response = requests.get(f"{url}/api/admin-roles", headers=headers, params=filtered_params)
        if response.status_code != 200:
            print(f"Error: HTTP {response.status_code} - {response.text}")
            exit(1)

        response_json = response.json()
        if "error" in response_json:
            print(f"Error: {response_json['error']}")
            exit(1)

        data = response_json.get("data", [])
        roles.extend(data)

        total = response_json.get("total", 0)
        if len(data) < pageSize or current * pageSize >= total:
            break

    return roles


def get_admin_role_by_guid(url, token, guid):
    """Get admin role by GUID"""
    headers = {"Authorization": f"Bearer {token}"}
    response = requests.get(f"{url}/api/admin-roles/{guid}", headers=headers)
    return check_response(response)


def get_admin_role_by_name(url, token, name):
    """Get admin role by name"""
    roles = view_admin_roles(url, token, name)
    for role in roles:
        if role["name"] == name:
            return role
    return None


def update_admin_role(url, token, guid, name=None, note=None, user_groups=None, device_groups=None, unassigned=None):
    """Update an admin role"""
    print(f"Updating admin role {guid}")
    headers = {"Authorization": f"Bearer {token}"}

    # Check if at least one parameter is provided for update
    update_params = [name, note, user_groups, device_groups, unassigned]
    if all(param is None for param in update_params):
        return "Error: At least one parameter must be specified for update"

    payload = {}

    if name is not None:
        payload["name"] = name
    if note is not None:
        payload["note"] = note
    if user_groups is not None:
        payload["user_groups"] = user_groups if isinstance(user_groups, list) else [user_groups]
    if device_groups is not None:
        payload["device_groups"] = device_groups if isinstance(device_groups, list) else [device_groups]
    if unassigned is not None:
        payload["unassigned"] = unassigned

    response = requests.put(f"{url}/api/admin-roles/{guid}", headers=headers, json=payload)
    return check_response(response)


def delete_admin_roles(url, token, guids):
    """Delete admin roles"""
    if isinstance(guids, str):
        guids = [guids]

    print(f"Deleting admin roles {guids}")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {"guids": guids}
    response = requests.delete(f"{url}/api/admin-roles", headers=headers, json=payload)
    return check_response(response)


def add_users_to_admin_role(url, token, role_guid, user_guids):
    """Add users to admin role"""
    if isinstance(user_guids, str):
        user_guids = [user_guids]

    print(f"Adding users {user_guids} to admin role {role_guid}")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {"users": user_guids}
    response = requests.post(f"{url}/api/admin-roles/{role_guid}/users", headers=headers, json=payload)
    return check_response(response)


def remove_users_from_admin_role(url, token, role_guid, user_guids):
    """Remove users from admin role"""
    if isinstance(user_guids, str):
        user_guids = [user_guids]

    print(f"Removing users {user_guids} from admin role {role_guid}")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {"users": user_guids}
    response = requests.delete(f"{url}/api/admin-roles/{role_guid}/users", headers=headers, json=payload)
    return check_response(response)


def main():
    parser = argparse.ArgumentParser(description="Admin Roles manager")

    # Required arguments
    parser.add_argument(
        "command",
        choices=["view", "get", "update", "delete", "add-users", "remove-users"],
        help="Command to execute",
    )

    # Global arguments (used by all commands)
    parser.add_argument("--url", required=True, help="URL of the API")
    parser.add_argument("--token", required=True, help="Bearer token for authentication")

    # Role identification
    parser.add_argument("--name", help="Admin role name (for identification or filtering)")
    parser.add_argument("--guid", help="Admin role GUID (alternative to name)")

    # Role management arguments
    parser.add_argument("--update-name", help="New admin role name (for update)")
    parser.add_argument("--note", help="Note field")
    parser.add_argument("--type", type=int, choices=[1, 2, 3], help="Role type (1=Global, 2=Individual, 3=GroupScoped)")
    parser.add_argument("--user-groups", help="User groups (comma-separated list of group names)")
    parser.add_argument("--device-groups", help="Device groups (comma-separated list of group names)")
    parser.add_argument("--unassigned", type=lambda x: x.lower() == 'true', help="Unassigned devices flag (true/false)")

    # User management arguments
    parser.add_argument("--user-guids", help="User GUIDs (comma-separated list)")

    args = parser.parse_args()

    # Remove trailing slashes from URL
    while args.url.endswith("/"):
        args.url = args.url[:-1]

    if args.command == "view":
        # View all admin roles
        roles = view_admin_roles(args.url, args.token, args.name, args.type)
        print(json.dumps(roles, indent=2))

    elif args.command == "get":
        # Get admin role by name or GUID
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for get command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        if args.guid:
            role = get_admin_role_by_guid(args.url, args.token, args.guid)
        else:
            role = get_admin_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Admin role '{args.name}' not found")
                return

        print(json.dumps(role, indent=2))

    elif args.command == "update":
        # Update admin role
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for update command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_admin_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Admin role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        # Parse groups if provided
        user_groups = None
        if args.user_groups:
            user_groups = [g.strip() for g in args.user_groups.split(",")]

        device_groups = None
        if args.device_groups:
            device_groups = [g.strip() for g in args.device_groups.split(",")]

        result = update_admin_role(
            args.url,
            args.token,
            guid,
            args.update_name,
            args.note,
            user_groups,
            device_groups,
            args.unassigned
        )
        print(f"Result: {result}")

    elif args.command == "delete":
        # Delete admin roles
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for delete command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_admin_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Admin role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        result = delete_admin_roles(args.url, args.token, guid)
        print(f"Result: {result}")

    elif args.command == "add-users":
        # Add users to admin role
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for add-users command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        if not args.user_guids:
            print("Error: --user-guids is required for add-users command")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_admin_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Admin role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        # Parse user GUIDs
        user_guids = [g.strip() for g in args.user_guids.split(",")]

        result = add_users_to_admin_role(args.url, args.token, guid, user_guids)
        print(f"Result: {result}")

    elif args.command == "remove-users":
        # Remove users from admin role
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for remove-users command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        if not args.user_guids:
            print("Error: --user-guids is required for remove-users command")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_admin_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Admin role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        # Parse user GUIDs
        user_guids = [g.strip() for g in args.user_guids.split(",")]

        result = remove_users_from_admin_role(args.url, args.token, guid, user_guids)
        print(f"Result: {result}")


if __name__ == "__main__":
    main()
