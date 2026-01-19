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


def view_control_roles(url, token, name=None, status=None):
    """View all control roles"""
    headers = {"Authorization": f"Bearer {token}"}
    pageSize = 30
    params = {}

    if name:
        params["name"] = name
    if status is not None:
        params["status"] = status

    filtered_params = {
        k: "%" + v + "%" if (v != "-" and "%" not in v and k != "status") else v
        for k, v in params.items()
        if v is not None
    }
    filtered_params["pageSize"] = pageSize

    roles = []
    current = 0

    while True:
        current += 1
        filtered_params["current"] = current
        response = requests.get(f"{url}/api/control-roles", headers=headers, params=filtered_params)
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


def get_control_role_by_guid(url, token, guid):
    """Get control role by GUID"""
    headers = {"Authorization": f"Bearer {token}"}
    response = requests.get(f"{url}/api/control-roles/{guid}", headers=headers)
    return check_response(response)


def get_control_role_by_name(url, token, name):
    """Get control role by name"""
    roles = view_control_roles(url, token, name)
    for role in roles:
        if role["name"] == name:
            return role
    return None


def add_control_role(url, token, name, note=None, proto=None):
    """Add a new control role"""
    print(f"Adding control role '{name}'")
    headers = {"Authorization": f"Bearer {token}"}

    payload = {
        "name": name,
    }

    if note:
        payload["note"] = note
    if proto:
        # proto is expected to be a base64-encoded protobuf bytes
        payload["info"] = {
            "proto": proto
        }

    response = requests.post(f"{url}/api/control-roles", headers=headers, json=payload)
    return check_response(response)


def update_control_role(url, token, guid, name=None, note=None, proto=None):
    """Update a control role"""
    print(f"Updating control role {guid}")
    headers = {"Authorization": f"Bearer {token}"}

    # Check if at least one parameter is provided for update
    update_params = [name, note, proto]
    if all(param is None for param in update_params):
        return "Error: At least one parameter must be specified for update"

    payload = {}

    if name is not None:
        payload["name"] = name
    if note is not None:
        payload["note"] = note
    if proto is not None:
        # proto is expected to be a base64-encoded protobuf bytes
        payload["info"] = {
            "proto": proto
        }

    response = requests.put(f"{url}/api/control-roles/{guid}", headers=headers, json=payload)
    return check_response(response)


def delete_control_roles(url, token, guids):
    """Delete control roles"""
    if isinstance(guids, str):
        guids = [guids]

    print(f"Deleting control roles {guids}")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {"guids": guids}
    response = requests.delete(f"{url}/api/control-roles", headers=headers, json=payload)
    return check_response(response)


def enable_control_role(url, token, guids, disable=False):
    """Enable or disable control roles"""
    if isinstance(guids, str):
        guids = [guids]

    action = "Disabling" if disable else "Enabling"
    print(f"{action} control roles {guids}")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {
        "guids": guids,
        "disable": disable
    }
    response = requests.post(f"{url}/api/control-roles/enable", headers=headers, json=payload)
    return check_response(response)


def assign_users_to_control_role(url, token, role_guid, user_guids):
    """Assign users to control role"""
    if isinstance(user_guids, str):
        user_guids = [user_guids]

    print(f"Assigning users {user_guids} to control role {role_guid}")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {"user_guids": user_guids}
    response = requests.post(f"{url}/api/control-roles/{role_guid}/users", headers=headers, json=payload)
    return check_response(response)


def remove_users_from_control_role(url, token, user_guids):
    """Remove users from control role (unassign)"""
    if isinstance(user_guids, str):
        user_guids = [user_guids]

    print(f"Removing users {user_guids} from control role")
    headers = {"Authorization": f"Bearer {token}"}
    payload = {"user_guids": user_guids}
    response = requests.delete(f"{url}/api/control-roles/users", headers=headers, json=payload)
    return check_response(response)


def main():
    parser = argparse.ArgumentParser(description="Control Roles manager")

    # Required arguments
    parser.add_argument(
        "command",
        choices=["view", "get", "add", "update", "delete", "enable", "disable", "assign-users", "remove-users"],
        help="Command to execute",
    )

    # Global arguments (used by all commands)
    parser.add_argument("--url", required=True, help="URL of the API")
    parser.add_argument("--token", required=True, help="Bearer token for authentication")

    # Role identification
    parser.add_argument("--name", help="Control role name (for identification or filtering)")
    parser.add_argument("--guid", help="Control role GUID (alternative to name)")

    # Role management arguments
    parser.add_argument("--update-name", help="New control role name (for update)")
    parser.add_argument("--note", help="Note field")
    parser.add_argument("--proto", help="Protobuf data (base64-encoded bytes)")
    parser.add_argument("--status", type=int, choices=[0, 1], help="Status filter (0=Disabled, 1=Enabled)")

    # User management arguments
    parser.add_argument("--user-guids", help="User GUIDs (comma-separated list)")

    args = parser.parse_args()

    # Remove trailing slashes from URL
    while args.url.endswith("/"):
        args.url = args.url[:-1]

    if args.command == "view":
        # View all control roles
        roles = view_control_roles(args.url, args.token, args.name, args.status)
        print(json.dumps(roles, indent=2))

    elif args.command == "get":
        # Get control role by name or GUID
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for get command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        if args.guid:
            role = get_control_role_by_guid(args.url, args.token, args.guid)
        else:
            role = get_control_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Control role '{args.name}' not found")
                return

        print(json.dumps(role, indent=2))

    elif args.command == "add":
        # Add new control role
        if not args.name:
            print("Error: --name is required for add command")
            return

        result = add_control_role(
            args.url,
            args.token,
            args.name,
            args.note,
            args.proto
        )
        print(f"Result: {result}")

    elif args.command == "update":
        # Update control role
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for update command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_control_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Control role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        result = update_control_role(
            args.url,
            args.token,
            guid,
            args.update_name,
            args.note,
            args.proto
        )
        print(f"Result: {result}")

    elif args.command == "delete":
        # Delete control roles
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for delete command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_control_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Control role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        result = delete_control_roles(args.url, args.token, guid)
        print(f"Result: {result}")

    elif args.command in ["enable", "disable"]:
        # Enable or disable control roles
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for enable/disable command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_control_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Control role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        disable = (args.command == "disable")
        result = enable_control_role(args.url, args.token, guid, disable)
        print(f"Result: {result}")

    elif args.command == "assign-users":
        # Assign users to control role
        if not args.name and not args.guid:
            print("Error: --name or --guid is required for assign-users command")
            return

        if args.name and args.guid:
            print("Error: Cannot specify both --name and --guid")
            return

        if not args.user_guids:
            print("Error: --user-guids is required for assign-users command")
            return

        # Get role GUID if name is provided
        if args.name:
            role = get_control_role_by_name(args.url, args.token, args.name)
            if not role:
                print(f"Error: Control role '{args.name}' not found")
                return
            guid = role["guid"]
        else:
            guid = args.guid

        # Parse user GUIDs
        user_guids = [g.strip() for g in args.user_guids.split(",")]

        result = assign_users_to_control_role(args.url, args.token, guid, user_guids)
        print(f"Result: {result}")

    elif args.command == "remove-users":
        # Remove users from control role
        if not args.user_guids:
            print("Error: --user-guids is required for remove-users command")
            return

        # Parse user GUIDs
        user_guids = [g.strip() for g in args.user_guids.split(",")]

        result = remove_users_from_control_role(args.url, args.token, user_guids)
        print(f"Result: {result}")


if __name__ == "__main__":
    main()
