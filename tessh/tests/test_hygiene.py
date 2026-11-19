def test_swap_does_not_capture(run):
    ns, out = run("""defmacro swap a b
  quasi
    tmp = $~a
    ~a = $~b
    ~b = $tmp
tmp = 1
other = 2
swap tmp other
print $tmp $other""")
    assert out == "2 1\n"


def test_macro_binding_does_not_clobber_user_variable(run):
    ns, out = run("""defmacro my_or a b
  quasi
    t = ~a
    if $t then $t else ~b
t = 5
r = my_or 0 $t""")
    assert ns["r"] == 5 and ns["t"] == 5


def test_free_names_mean_the_definition_site(run):
    ns, out = run("""defmacro mklist x
  :(list ~x)
def f
  list = 7
  mklist "ab"
r = (f)""")
    assert ns["r"] == ["a", "b"]


def test_capture_is_unhygienic_on_purpose(run):
    _, out = run("""defmacro aif cond:expr *body
  it = capture "it"
  quasi
    ~it = ~cond
    if $~it
      ~@body
aif (len [1 2 3])
  print got $it""")
    assert out == "got 3\n"


def test_expr_parameter_and_splice(run):
    _, out = run("""defmacro unless cond:expr *body
  quasi
    if (not ~cond)
      ~@body
x = 2
unless $x > 3
  print a
  print b""")
    assert out == "a\nb\n"


def test_nested_macros_keep_hygiene(run):
    ns, out = run("""defmacro inc v
  quasi
    tmp = $~v + 1
    ~v = $tmp
defmacro inc2 v
  quasi
    inc ~v
    inc ~v
tmp = 10
n = 0
inc2 n
print $n $tmp""")
    assert out == "2 10\n"


def test_gensym_and_lift(run):
    ns, _ = run("""defmacro const3
  quasi
    ~(1 + 2)
r = const3""")
    assert ns["r"] == 3


def test_macro_errors_have_positions():
    from tessh.runner import check
    (e,) = check("defmacro m a\n  quasi\n    ~a\nm\n")
    assert e[0] == 4 and "needs a" in e[3]
