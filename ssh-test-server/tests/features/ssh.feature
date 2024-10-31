Feature: User can execute some dummy command over ssh server

  Scenario Outline: Execute remote command
    Given Running ssh server
    When Executed command '<command>' on remote ssh server
    Then Got exit code <status_code> and response '<stdout>' and error containing '<stderr>'
    Examples:
      | command    | status_code | stdout | stderr                   |
      | echo abc   | 0           | abc    |                          |
      | echo "a b" | 0           | a b    |                          |
      | nocmd      | 127         |        | nocmd: command not found |
      | exit       | 0           |        |                          |


#  Scenario: Change remote password for a user
#    Given Running ssh server
#    When Change user password via passwd command
#    Then Password has been changed

#  Scenario: Cannot login with wrong password
#    Given Running ssh server
#    When Tried to login with wrong password
#    Then Failed to login
