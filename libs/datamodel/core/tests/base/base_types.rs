use crate::common::*;
use datamodel::ast::Span;
use datamodel::error::DatamodelError;
use datamodel::{common::ScalarType, dml};

#[test]
fn parse_scalar_types() {
    let dml = r#"
    model User {
        id Int @id
        firstName String
        age Int
        isPro Boolean
        balance Decimal
        averageGrade Float
    }
    "#;

    let schema = parse(dml);
    let user_model = schema.assert_has_model("User");
    user_model
        .assert_has_field("firstName")
        .assert_base_type(&ScalarType::String);
    user_model.assert_has_field("age").assert_base_type(&ScalarType::Int);
    user_model
        .assert_has_field("isPro")
        .assert_base_type(&ScalarType::Boolean);
    user_model
        .assert_has_field("balance")
        .assert_base_type(&ScalarType::Decimal);
    user_model
        .assert_has_field("averageGrade")
        .assert_base_type(&ScalarType::Float);
}

#[test]
fn parse_field_arity() {
    let dml = r#"
    datasource mypg {
        provider = "postgres"
        url = "postgresql://asdlj"
    }
    
    model Post {
        id Int @id
        text String
        photo String?
        comments String[]
        enums    Enum[]
    }
    
    enum Enum {
        A
        B
        C
    }
    "#;

    let schema = parse(dml);
    let post_model = schema.assert_has_model("Post");
    post_model
        .assert_has_field("text")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::Required);
    post_model
        .assert_has_field("photo")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::Optional);
    post_model
        .assert_has_field("comments")
        .assert_base_type(&ScalarType::String)
        .assert_arity(&dml::FieldArity::List);

    post_model
        .assert_has_field("enums")
        .assert_enum_type("Enum")
        .assert_arity(&dml::FieldArity::List);
}

#[test]
fn scalar_list_types_are_not_supported_by_default() {
    let dml = r#"
    model Post {
        id         Int @id
        text       String
        photo      String?
        comments   String[]
        enums      Enum[]
        categories Category[] // make sure that relations still work
    }
    
    enum Enum {
        A
        B
        C
    }
    
    model Category {
      id   Int    @id
      name String
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_length(2);

    errors.assert_is_at(
        0,
        DatamodelError::new_scalar_list_fields_are_not_supported("Post", "comments", Span::new(106, 125)),
    );

    errors.assert_is_at(
        1,
        DatamodelError::new_scalar_list_fields_are_not_supported("Post", "enums", Span::new(134, 151)),
    );
}

#[test]
fn scalar_list_types_are_not_supported_by_mysql() {
    let dml = r#"
    datasource mysql {
        provider = "mysql"
        url = "mysql://asdlj"
    }
    
    model Post {
        id Int @id
        text String
        photo String?
        comments String[]
        enums    Enum[]
    }
    
    enum Enum {
        A
        B
        C
    }
    "#;

    let errors = parse_error(dml);

    errors.assert_length(2);

    errors.assert_is_at(
        0,
        DatamodelError::new_scalar_list_fields_are_not_supported("Post", "comments", Span::new(178, 195)),
    );

    errors.assert_is_at(
        1,
        DatamodelError::new_scalar_list_fields_are_not_supported("Post", "enums", Span::new(204, 219)),
    );
}
