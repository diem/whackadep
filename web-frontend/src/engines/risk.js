export function calculate_risk_score(dep) {
  let risk_score = 0;
  let risk_reasons = [];

  if (dep.update.build_rs) {
    risk_score += 10;
    risk_reasons.push("<code>build.rs</code> file changed");
  }

  return { risk_score, risk_reasons };
}
