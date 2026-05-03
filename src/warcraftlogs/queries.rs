pub const GET_CHARACTER_RANKINGS: &str = r#"
  query GetCharacterRankings(
    $name: String!
    $serverSlug: String!
    $serverRegion: String!
    $zoneID: Int
  ) {
    characterData {
      character(
        name: $name
        serverSlug: $serverSlug
        serverRegion: $serverRegion
      ) {
        id
        name
        classID
        server {
          name
          slug
          region {
            name
            slug
          }
        }
        zoneRankings(zoneID: $zoneID)
      }
    }
  }
"#;

pub const GET_CHARACTER_RECENT_REPORT: &str = r#"
  query GetCharacterRecentReport(
    $name: String!
    $serverSlug: String!
    $serverRegion: String!
  ) {
    characterData {
      character(
        name: $name
        serverSlug: $serverSlug
        serverRegion: $serverRegion
      ) {
        recentReports(limit: 1) {
          data {
            code
            startTime
            fights(killType: Kills) {
              id
              name
              encounterID
            }
          }
        }
      }
    }
  }
"#;

pub const GET_REPORT_MASTER_DATA: &str = r#"
  query GetReportMasterData($reportCode: String!) {
    reportData {
      report(code: $reportCode) {
        masterData(translate: true) {
          actors {
            id
            name
            type
            subType
          }
        }
      }
    }
  }
"#;

pub const GET_REPORT_COMBATANT_INFO: &str = r#"
  query GetReportCombatantInfo(
    $reportCode: String!
    $fightID: Int!
    $sourceID: Int!
  ) {
    reportData {
      report(code: $reportCode) {
        events(
          dataType: CombatantInfo
          fightIDs: [$fightID]
          sourceID: $sourceID
          limit: 1
          translate: true
        ) {
          data
        }
      }
    }
  }
"#;
