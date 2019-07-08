use bson::{Bson, Document};
use mongodb::options::{collation::Collation, AggregateOptions};

use super::{Outcome, TestFile};
use crate::CLIENT;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Arguments {
    pub pipeline: Vec<Document>,
    pub batch_size: Option<i32>,
    pub collation: Option<Collation>,
}

#[function_name]
fn run_aggregate_test(test_file: TestFile) {
    let data = test_file.data;

    for mut test_case in test_file.tests {
        if test_case.operation.name != "aggregate" {
            continue;
        }

        test_case.description = test_case.description.replace('$', "%");

        let coll = CLIENT.init_db_and_coll(function_name!(), &test_case.description);
        coll.insert_many(data.clone(), None)
            .expect(&test_case.description);

        let arguments: Arguments = bson::from_bson(Bson::Document(test_case.operation.arguments))
            .expect(&test_case.description);
        let outcome: Outcome<Option<Vec<Document>>> =
            bson::from_bson(Bson::Document(test_case.outcome)).expect(&test_case.description);

        if let Some(ref c) = outcome.collection {
            if let Some(ref name) = c.name {
                CLIENT
                    .get_coll(function_name!(), name)
                    .drop()
                    .expect(&test_case.description);
            }
        }

        let options = AggregateOptions {
            batch_size: arguments.batch_size,
            collation: arguments.collation,
            ..Default::default()
        };

        {
            let cursor = coll
                .aggregate(arguments.pipeline, Some(options))
                .expect(&test_case.description);
            assert_eq!(
                outcome.result.unwrap_or_default(),
                cursor.map(Result::unwrap).collect::<Vec<_>>(),
                "{}",
                test_case.description,
            );
        }

        if let Some(c) = outcome.collection {
            let outcome_coll = match c.name {
                Some(ref name) => CLIENT.get_coll(function_name!(), name),
                None => coll,
            };

            assert_eq!(
                c.data,
                super::find_all(&outcome_coll),
                "{}",
                test_case.description
            );
        }
    }
}

#[test]
fn run() {
    crate::spec::test(&["crud", "v1", "read"], run_aggregate_test);
}