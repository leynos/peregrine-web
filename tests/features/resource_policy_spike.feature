Feature: Resource identity survives async admission
  Scenario: Public resource permits direct invocation
    Given a public resource with an injected dependency and no enforcer
    When the selected resource crosses an awaited hook
    Then that resource's responder runs once
    And its dependency supplies the response

  Scenario: Protected policy denies before invocation
    Given a protected resource and an enforcer returning denial
    When its frozen policy and method cross an awaited hook
    Then the selected resource is unchanged
    And no responder runs

  Scenario: Protected registration requires enforcement
    Given a protected resource without an enforcer
    When the specimen validates registration
    Then registration fails with a typed missing-enforcer error

  Scenario: Present identity permits a protected invocation
    Given a protected resource and an enforcer returning permission
    When its frozen policy and method cross an awaited hook
    Then that resource's responder runs once
    And its dependency supplies the response

  Scenario: Absent identity is denied before the hook
    Given a protected resource and an enforcer returning permission
    When an anonymous request crosses the admission boundary
    Then the request is denied for a missing identity
    And no responder runs
