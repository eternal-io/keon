mod util;

#[test]
fn backwards() {
    util::backward(
        &String::from("To be, or not to be, that is the question."),
        "| To be, or not to be,
         | that is the question.",
    )
    .unwrap();

    util::backward(
        &String::from("To be, or not to be, that is the question:\n"),
        "| To be, or not to be,
         | that is the question:
         |",
    )
    .unwrap();

    util::backward(
        &String::from("我能吞下玻璃 而不伤身体。"),
        "| 我能吞下玻璃
         | 而不伤身体。",
    )
    .unwrap();

    util::backward(
        &String::from("我能吞下玻璃而不伤身体。"),
        "| 我能吞下玻璃
         < 而不伤身体。",
    )
    .unwrap();

    util::backward(
        &String::from(
            "#include<iostream>
using namespace std;
int main() {
    cout << ... << endl;
    return 0;
}",
        ),
        "| #include<iostream>
         ` using namespace std;
         ` int main() {
         `     cout << ... << endl;
         `     return 0;
         ` }",
    )
    .unwrap();
}
