export function calculate_risk_score(dep) {
  var risk_score = 0;
  var risk_reasons = [];

  if (dep.update.build_rs) {
    risk_score += 10;
    risk_reasons.push("<code>build.rs</code> file Changed");
  }

  return { risk_score, risk_reasons };
}
