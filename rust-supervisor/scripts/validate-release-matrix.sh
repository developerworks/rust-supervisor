#!/usr/bin/env bash
# Validates the release gate matrix CSV format per
# specs/006-8-product-bundle-runbooks/contracts/release-matrix-format.md.
set -euo pipefail

MATRIX_FILE="artifacts/quality-gate-outcome.csv"
PASS=true
BLOCKING=0

if [ ! -f "$MATRIX_FILE" ]; then
    echo "FAIL: $MATRIX_FILE not found"
    exit 1
fi

echo "=== Release Matrix Validation ==="

# 1. Check header columns
HEADER=$(head -1 "$MATRIX_FILE")
EXPECTED_HEADER="gate_id,tier,outcome,detail,exemption_ticket,exemption_url,log_path,timestamp"
if [ "$HEADER" = "$EXPECTED_HEADER" ]; then
    echo "PASS: Header columns match expected"
else
    echo "WARN: Header mismatch"
    echo "  Expected: $EXPECTED_HEADER"
    echo "  Got:      $HEADER"
fi

# 2. Validate each data row (skip header)
LINE_NUM=1
tail -n +2 "$MATRIX_FILE" | while IFS=',' read -r gate_id tier outcome detail exemption_ticket exemption_url log_path timestamp; do
    LINE_NUM=$((LINE_NUM + 1))

    # Skip rows with empty outcome (not yet filled)
    if [ -z "$outcome" ]; then
        continue
    fi

    # 3. Validate outcome values
    case "$outcome" in
        passed|failed|waived|skipped|missing)
            echo "PASS: Line $LINE_NUM ($gate_id): outcome=$outcome"
            ;;
        *)
            echo "FAIL: Line $LINE_NUM ($gate_id): invalid outcome '$outcome'"
            PASS=false
            ;;
    esac

    # 4. Check blank exemption_ticket + non-passing outcome = blocking
    if [ "$outcome" != "passed" ] && [ -z "$exemption_ticket" ]; then
        echo "BLOCKING: Line $LINE_NUM ($gate_id): outcome=$outcome but no exemption_ticket"
        BLOCKING=$((BLOCKING + 1))
    fi
done

# 5. Generate HTML table from CSV for release page
HTML_OUTPUT="artifacts/release-matrix.html"
echo "<table>" > "$HTML_OUTPUT"
echo "<tr><th>Gate</th><th>Tier</th><th>Outcome</th><th>Exemption</th><th>Archive</th></tr>" >> "$HTML_OUTPUT"
tail -n +2 "$MATRIX_FILE" | while IFS=',' read -r gate_id tier outcome _ exemption_ticket _ log_path _; do
    echo "<tr>" >> "$HTML_OUTPUT"
    echo "  <td>$gate_id</td>" >> "$HTML_OUTPUT"
    echo "  <td>$tier</td>" >> "$HTML_OUTPUT"
    case "$outcome" in
        passed)  echo "  <td>&#9989;</td>" >> "$HTML_OUTPUT" ;;
        failed)  echo "  <td>&#10060;</td>" >> "$HTML_OUTPUT" ;;
        skipped) echo "  <td>&#8212;</td>" >> "$HTML_OUTPUT" ;;
        waived)  echo "  <td>&#9888;</td>" >> "$HTML_OUTPUT" ;;
        missing) echo "  <td>&#10067;</td>" >> "$HTML_OUTPUT" ;;
        *)       echo "  <td></td>" >> "$HTML_OUTPUT" ;;
    esac
    if [ -n "$exemption_ticket" ]; then
        echo "  <td>$exemption_ticket</td>" >> "$HTML_OUTPUT"
    else
        echo "  <td></td>" >> "$HTML_OUTPUT"
    fi
    if [ -n "$log_path" ]; then
        echo "  <td>$log_path</td>" >> "$HTML_OUTPUT"
    else
        echo "  <td></td>" >> "$HTML_OUTPUT"
    fi
    echo "</tr>" >> "$HTML_OUTPUT"
done
echo "</table>" >> "$HTML_OUTPUT"

# 6. Check for empty <td></td> in generated HTML
EMPTY_TD=$(grep -c '<td></td>' "$HTML_OUTPUT" || true)
if [ "$EMPTY_TD" -gt 0 ]; then
    echo "WARN: $EMPTY_TD empty <td> cells in generated HTML"
fi

echo "---"
if [ "$BLOCKING" -gt 0 ]; then
    echo "BLOCKING: $BLOCKING gate(s) with non-passing outcome and no exemption ticket"
    PASS=false
fi
if [ "$PASS" = true ]; then
    echo "Result: PASS"
    echo "HTML table written to $HTML_OUTPUT"
    exit 0
else
    echo "Result: FAIL"
    exit 1
fi
