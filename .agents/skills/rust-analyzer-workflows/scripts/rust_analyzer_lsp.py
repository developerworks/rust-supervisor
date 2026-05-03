#!/usr/bin/env python3
"""Send one or two LSP requests to rust-analyzer and print the JSON result body."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path


TEXT_DOCUMENT_METHODS = {
    "diagnostic",
    "codeAction",
    "completion",
    "definition",
    "documentHighlight",
    "documentSymbol",
    "expandMacro",
    "externalDocs",
    "foldingRange",
    "formatting",
    "getFailedObligations",
    "hover",
    "implementation",
    "inlayHint",
    "interpretFunction",
    "openCargoToml",
    "parentModule",
    "prepareCallHierarchy",
    "prepareRename",
    "references",
    "relatedTests",
    "rename",
    "runnables",
    "selectionRange",
    "semanticTokensFull",
    "signatureHelp",
    "typeDefinition",
    "viewFileText",
    "viewHir",
    "viewItemTree",
    "viewMir",
    "viewRecursiveMemoryLayout",
    "viewSyntaxTree",
}

POSITION_METHODS = {
    "completion",
    "definition",
    "expandMacro",
    "externalDocs",
    "documentHighlight",
    "getFailedObligations",
    "hover",
    "implementation",
    "interpretFunction",
    "openCargoToml",
    "parentModule",
    "prepareCallHierarchy",
    "prepareRename",
    "references",
    "relatedTests",
    "rename",
    "runnables",
    "selectionRange",
    "signatureHelp",
    "typeDefinition",
    "viewHir",
    "viewMir",
    "viewRecursiveMemoryLayout",
}

RANGE_METHODS = {"codeAction", "inlayHint"}

METHOD_CHOICES = sorted(
    TEXT_DOCUMENT_METHODS
    | {
        "analyzerStatus",
        "fetchDependencyList",
        "incomingCalls",
        "outgoingCalls",
        "rebuildProcMacros",
        "reloadWorkspace",
        "viewCrateGraph",
        "workspaceSymbol",
    }
)


def read_message(stream) -> dict:
    content_length = None
    while True:
        line = stream.readline()
        if not line:
            raise EOFError("unexpected EOF while reading LSP headers")
        if line == b"\r\n":
            break
        header = line.decode("utf-8").strip()
        if not header:
            break
        key, _, value = header.partition(":")
        if key.lower() == "content-length":
            content_length = int(value.strip())
    if content_length is None:
        raise ValueError("missing Content-Length header")
    body = stream.read(content_length)
    if len(body) != content_length:
        raise EOFError("unexpected EOF while reading LSP body")
    return json.loads(body.decode("utf-8"))


def send_message(stream, payload: dict) -> None:
    body = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
    stream.write(header)
    stream.write(body)
    stream.flush()


def perform_request(process: subprocess.Popen, method: str, params: dict, request_id: int) -> dict:
    assert process.stdin is not None
    assert process.stdout is not None
    send_message(
        process.stdin,
        {
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        },
    )
    while True:
        message = read_message(process.stdout)
        if "method" in message:
            if "id" in message:
                send_message(
                    process.stdin,
                    {
                        "jsonrpc": "2.0",
                        "id": message["id"],
                        "result": None,
                    },
                )
            continue
        if message.get("id") == request_id:
            return message


def relative_to_workspace(workspace: Path, file_path: Path | None) -> str | None:
    if file_path is None:
        return None
    try:
        return file_path.resolve().relative_to(workspace.resolve()).as_posix()
    except ValueError:
        return file_path.resolve().as_posix()


def build_client_capabilities() -> dict:
    return {
        "experimental": {
            "codeActionGroup": True,
            "colorDiagnosticOutput": True,
            "hoverActions": True,
            "localDocs": True,
            "serverStatusNotification": True,
            "snippetTextEdit": True,
            "testExplorer": True,
        },
        "textDocument": {
            "codeAction": {
                "codeActionLiteralSupport": {
                    "codeActionKind": {
                        "valueSet": [
                            "",
                            "quickfix",
                            "refactor",
                            "refactor.extract",
                            "refactor.inline",
                            "refactor.rewrite",
                        ]
                    }
                }
            },
            "completion": {
                "completionItem": {
                    "snippetSupport": True,
                    "resolveSupport": {
                        "properties": [
                            "additionalTextEdits",
                            "detail",
                            "documentation",
                        ]
                    },
                }
            },
            "diagnostic": {},
            "semanticTokens": {
                "requests": {
                    "range": True,
                    "full": {"delta": True},
                },
                "formats": ["relative"],
                "tokenTypes": [],
                "tokenModifiers": [],
            },
            "signatureHelp": {
                "signatureInformation": {
                    "parameterInformation": {"labelOffsetSupport": True}
                }
            },
        },
        "workspace": {
            "diagnostics": {
                "refreshSupport": True,
            },
            "symbol": {
                "resolveSupport": {"properties": ["location.range"]}
            }
        }
    }


def build_position(args: argparse.Namespace) -> dict:
    return {"line": args.line, "character": args.character}


def build_range(args: argparse.Namespace) -> dict:
    end_line = args.end_line if args.end_line is not None else args.line
    end_character = args.end_character if args.end_character is not None else args.character
    return {
        "start": {"line": args.line, "character": args.character},
        "end": {"line": end_line, "character": end_character},
    }


def build_request(
    args: argparse.Namespace,
    file_uri: str | None,
) -> tuple[str, dict]:
    position = build_position(args)
    text_document = {"uri": file_uri}
    position_params = {
        "textDocument": text_document,
        "position": position,
    }
    if args.method == "definition":
        return "textDocument/definition", position_params
    if args.method == "references":
        return "textDocument/references", {
            **position_params,
            "context": {"includeDeclaration": args.include_declaration},
        }
    if args.method == "diagnostic":
        return "textDocument/diagnostic", {
            "textDocument": text_document,
            "identifier": "rust-analyzer",
        }
    if args.method == "documentSymbol":
        return "textDocument/documentSymbol", {"textDocument": text_document}
    if args.method == "hover":
        return "textDocument/hover", position_params
    if args.method == "prepareRename":
        return "textDocument/prepareRename", position_params
    if args.method == "rename":
        return "textDocument/rename", {
            **position_params,
            "newName": args.new_name,
        }
    if args.method == "completion":
        params = {
            **position_params,
        }
        if args.trigger_character:
            params["context"] = {
                "triggerKind": 2,
                "triggerCharacter": args.trigger_character,
            }
        return "textDocument/completion", params
    if args.method == "signatureHelp":
        return "textDocument/signatureHelp", position_params
    if args.method == "inlayHint":
        return "textDocument/inlayHint", {
            "textDocument": text_document,
            "range": build_range(args),
        }
    if args.method == "workspaceSymbol":
        params = {"query": args.query}
        if args.symbol_scope:
            params["searchScope"] = args.symbol_scope
        if args.symbol_kind:
            params["searchKind"] = args.symbol_kind
        return "workspace/symbol", params
    if args.method == "typeDefinition":
        return "textDocument/typeDefinition", position_params
    if args.method == "implementation":
        return "textDocument/implementation", position_params
    if args.method == "documentHighlight":
        return "textDocument/documentHighlight", position_params
    if args.method == "selectionRange":
        return "textDocument/selectionRange", {
            "textDocument": text_document,
            "positions": [position],
        }
    if args.method == "foldingRange":
        return "textDocument/foldingRange", {"textDocument": text_document}
    if args.method == "semanticTokensFull":
        return "textDocument/semanticTokens/full", {
            "textDocument": text_document
        }
    if args.method == "formatting":
        return "textDocument/formatting", {
            "textDocument": text_document,
            "options": {
                "tabSize": args.tab_size,
                "insertSpaces": not args.use_tabs,
            },
        }
    if args.method == "codeAction":
        context = {"diagnostics": []}
        if args.only:
            context["only"] = args.only
        return "textDocument/codeAction", {
            "textDocument": text_document,
            "range": build_range(args),
            "context": context,
        }
    if args.method == "prepareCallHierarchy":
        return "textDocument/prepareCallHierarchy", position_params
    if args.method == "expandMacro":
        return "rust-analyzer/expandMacro", position_params
    if args.method == "externalDocs":
        return "experimental/externalDocs", position_params
    if args.method == "getFailedObligations":
        return "rust-analyzer/getFailedObligations", position_params
    if args.method == "parentModule":
        return "experimental/parentModule", position_params
    if args.method == "interpretFunction":
        return "rust-analyzer/interpretFunction", position_params
    if args.method == "openCargoToml":
        return "experimental/openCargoToml", position_params
    if args.method == "relatedTests":
        return "rust-analyzer/relatedTests", position_params
    if args.method == "runnables":
        return "experimental/runnables", position_params
    if args.method == "viewSyntaxTree":
        return "rust-analyzer/viewSyntaxTree", {"textDocument": text_document}
    if args.method == "viewHir":
        return "rust-analyzer/viewHir", position_params
    if args.method == "viewItemTree":
        return "rust-analyzer/viewItemTree", {"textDocument": text_document}
    if args.method == "viewMir":
        return "rust-analyzer/viewMir", position_params
    if args.method == "viewFileText":
        return "rust-analyzer/viewFileText", text_document
    if args.method == "viewRecursiveMemoryLayout":
        return "rust-analyzer/viewRecursiveMemoryLayout", position_params
    if args.method == "analyzerStatus":
        params = {}
        if file_uri is not None:
            params["textDocument"] = text_document
        return "rust-analyzer/analyzerStatus", params
    if args.method == "reloadWorkspace":
        return "rust-analyzer/reloadWorkspace", None
    if args.method == "rebuildProcMacros":
        return "rust-analyzer/rebuildProcMacros", None
    if args.method == "fetchDependencyList":
        return "rust-analyzer/fetchDependencyList", {}
    if args.method == "viewCrateGraph":
        return "rust-analyzer/viewCrateGraph", {"full": args.full_graph}
    raise ValueError(f"unsupported method: {args.method}")


def completion_items(result: object) -> list[dict]:
    if result is None:
        return []
    if isinstance(result, dict):
        items = result.get("items")
        return items if isinstance(items, list) else []
    if isinstance(result, list):
        return result
    return []


def should_retry_response(method: str, response: dict) -> bool:
    error_code = response.get("error", {}).get("code")
    if error_code == -32801:
        return True
    result = response.get("result")
    if result == []:
        return True
    if method == "completion" and completion_items(result) == []:
        return True
    return False


def validate_args(args: argparse.Namespace) -> None:
    file_required_methods = (
        TEXT_DOCUMENT_METHODS
        | {"incomingCalls", "outgoingCalls"}
        - {"analyzerStatus", "fetchDependencyList", "reloadWorkspace", "viewCrateGraph"}
    )
    if args.method in file_required_methods and not args.file:
        raise SystemExit(f"--file is required for --method {args.method}")
    if args.method == "workspaceSymbol" and not args.query:
        raise SystemExit("--query is required for --method workspaceSymbol")
    if args.method == "rename" and not args.new_name:
        raise SystemExit("--new-name is required for --method rename")
    if args.method == "completion" and args.resolve_index is not None and args.resolve_index < 0:
        raise SystemExit("--resolve-index must be >= 0")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Send one or two rust-analyzer LSP requests and print the JSON result.",
    )
    parser.add_argument("--workspace", required=True, help="Workspace root containing Cargo.toml")
    parser.add_argument(
        "--file",
        help="File path, absolute or workspace-relative. Required for textDocument methods.",
    )
    parser.add_argument(
        "--method",
        required=True,
        choices=METHOD_CHOICES,
        help="LSP method alias or workflow suffix",
    )
    parser.add_argument("--line", type=int, default=0, help="0-based line")
    parser.add_argument("--character", type=int, default=0, help="0-based character")
    parser.add_argument("--end-line", type=int, help="0-based end line for range requests")
    parser.add_argument(
        "--end-character",
        type=int,
        help="0-based end character for range requests",
    )
    parser.add_argument(
        "--include-declaration",
        action="store_true",
        help="Include declaration in references results",
    )
    parser.add_argument(
        "--new-name",
        help="New symbol name for textDocument/rename",
    )
    parser.add_argument(
        "--query",
        help="Query string for workspace/symbol",
    )
    parser.add_argument(
        "--symbol-scope",
        choices=["workspace", "workspaceAndDependencies"],
        help="Optional workspace/symbol search scope",
    )
    parser.add_argument(
        "--symbol-kind",
        choices=["onlyTypes", "allSymbols"],
        help="Optional workspace/symbol search kind filter",
    )
    parser.add_argument(
        "--only",
        action="append",
        help="Code action kind filter, e.g. refactor or quickfix",
    )
    parser.add_argument(
        "--tab-size",
        type=int,
        default=4,
        help="Formatting tab size for textDocument/formatting",
    )
    parser.add_argument(
        "--use-tabs",
        action="store_true",
        help="Use tabs for textDocument/formatting",
    )
    parser.add_argument(
        "--trigger-character",
        help="Completion trigger character, for example '.'",
    )
    parser.add_argument(
        "--resolve-index",
        type=int,
        help="For completion, also run completionItem/resolve on the selected item index",
    )
    parser.add_argument(
        "--full-graph",
        action="store_true",
        help="For viewCrateGraph, include dependency and sysroot crates",
    )
    parser.add_argument(
        "--delay-ms",
        type=int,
        default=3000,
        help="Milliseconds to wait after didOpen before sending the request",
    )
    parser.add_argument(
        "--retries",
        type=int,
        default=2,
        help="Retries for transient empty or content-modified semantic results",
    )
    parser.add_argument(
        "--retry-delay-ms",
        type=int,
        default=2000,
        help="Delay between retries in milliseconds",
    )
    args = parser.parse_args()

    workspace = Path(args.workspace).resolve()
    if not workspace.exists():
        raise SystemExit(f"workspace not found: {workspace}")

    validate_args(args)

    file_path: Path | None = None
    if args.file:
        file_path = Path(args.file)
        if not file_path.is_absolute():
            file_path = workspace / file_path
        file_path = file_path.resolve()
        if not file_path.exists():
            raise SystemExit(f"file not found: {file_path}")

    process = subprocess.Popen(
        ["rust-analyzer"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        cwd=workspace,
    )
    assert process.stdin is not None
    assert process.stdout is not None

    initialize = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": None,
            "rootUri": workspace.as_uri(),
            "capabilities": build_client_capabilities(),
            "clientInfo": {"name": "ra-lsp-request", "version": "2.0"},
            "workspaceFolders": [{"uri": workspace.as_uri(), "name": workspace.name}],
        },
    }
    send_message(process.stdin, initialize)

    initialize_result = read_message(process.stdout)
    if initialize_result.get("id") != 1:
        raise SystemExit(
            f"unexpected initialize response: {json.dumps(initialize_result, ensure_ascii=False)}"
        )

    send_message(process.stdin, {"jsonrpc": "2.0", "method": "initialized", "params": {}})

    file_uri = file_path.as_uri() if file_path is not None else None
    if file_path is not None:
        send_message(
            process.stdin,
            {
                "jsonrpc": "2.0",
                "method": "textDocument/didOpen",
                "params": {
                    "textDocument": {
                        "uri": file_uri,
                        "languageId": "rust",
                        "version": 1,
                        "text": file_path.read_text(encoding="utf-8"),
                    }
                },
            },
        )

    if args.delay_ms > 0 and file_path is not None:
        time.sleep(args.delay_ms / 1000.0)

    response = None
    resolved_item = None
    prepared_items = None
    request_id = 2
    attempts = args.retries + 1

    for attempt in range(attempts):
        if args.method in {"incomingCalls", "outgoingCalls"}:
            prepare_method, prepare_params = build_request(
                argparse.Namespace(**{**vars(args), "method": "prepareCallHierarchy"}),
                file_uri,
            )
            prepare_response = perform_request(process, prepare_method, prepare_params, request_id)
            request_id += 1
            prepared_items = prepare_response.get("result") or []
            if not prepared_items:
                response = {"jsonrpc": "2.0", "id": request_id, "result": []}
                break
            call_method = (
                "callHierarchy/incomingCalls"
                if args.method == "incomingCalls"
                else "callHierarchy/outgoingCalls"
            )
            response = perform_request(
                process,
                call_method,
                {"item": prepared_items[0]},
                request_id,
            )
        else:
            method_name, params = build_request(args, file_uri)
            response = perform_request(process, method_name, params, request_id)

        if not should_retry_response(args.method, response) or attempt + 1 >= attempts:
            break
        time.sleep(args.retry_delay_ms / 1000.0)
        request_id += 1

    if args.method == "completion" and args.resolve_index is not None and response is not None:
        items = completion_items(response.get("result"))
        if args.resolve_index < len(items):
            request_id += 1
            resolved_item = perform_request(
                process,
                "completionItem/resolve",
                items[args.resolve_index],
                request_id,
            )

    output = {
        "workspace": str(workspace),
        "file": relative_to_workspace(workspace, file_path),
        "method": args.method,
        "response": response,
    }
    if prepared_items is not None:
        output["prepared_items"] = prepared_items
    if resolved_item is not None:
        output["resolved_item"] = resolved_item
    print(json.dumps(output, ensure_ascii=False, indent=2))

    process.kill()
    try:
        process.wait(timeout=1)
    except subprocess.TimeoutExpired:
        process.terminate()

    return 0


if __name__ == "__main__":
    sys.exit(main())
