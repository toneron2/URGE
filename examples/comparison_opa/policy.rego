# OPA/Rego side of the comparison.
#
# Same scenario as the URGE side: a payment agent may run submit_payment only
# if it is authorized AND the compliance review is complete.
#
# Evaluate with:
#   opa eval -d policy.rego -i input_ok.json             'data.agent.gate.allow'
#   opa eval -d policy.rego -i input_review_missing.json 'data.agent.gate.allow'

package agent.gate

default allow := false

allow if {
	input.authorized
	input.review_completed
}
